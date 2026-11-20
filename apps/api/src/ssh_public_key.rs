use base64::{
    Engine as _,
    engine::general_purpose::{STANDARD, STANDARD_NO_PAD},
};
use sha2::{Digest, Sha256};

const MAX_BLOB_BYTES: usize = 8 * 1024;
const MAX_RSA_BITS: usize = 8 * 1024;

fn read_wire_string<'a>(
    blob: &'a [u8],
    offset: &mut usize,
) -> Result<&'a [u8], &'static str> {
    let length_end = (*offset).checked_add(4).ok_or("SSH key length overflow")?;
    let length_bytes = blob
        .get(*offset..length_end)
        .ok_or("SSH key field length is truncated")?;
    let length = u32::from_be_bytes(
        length_bytes
            .try_into()
            .map_err(|_| "SSH key field length is malformed")?,
    ) as usize;
    let value_end = length_end
        .checked_add(length)
        .ok_or("SSH key field length overflow")?;
    let value = blob
        .get(length_end..value_end)
        .ok_or("SSH key field is truncated")?;
    *offset = value_end;
    Ok(value)
}

fn positive_mpint(value: &[u8]) -> Result<&[u8], &'static str> {
    if value.is_empty() || value[0] & 0x80 != 0 {
        return Err("RSA key contains an invalid or negative integer");
    }

    let normalized = if value[0] == 0 {
        if value.len() < 2 || value[1] & 0x80 == 0 {
            return Err("RSA key contains a non-canonical integer");
        }
        &value[1..]
    } else {
        value
    };

    if normalized.is_empty() || normalized.iter().all(|byte| *byte == 0) {
        return Err("RSA key contains a zero integer");
    }
    Ok(normalized)
}

fn validate_application(value: &[u8]) -> Result<(), &'static str> {
    if value.is_empty() || value.len() > 1024 || value.iter().any(u8::is_ascii_control) {
        return Err("Security-key application is invalid");
    }
    Ok(())
}

fn validate_blob(key_type: &str, blob: &[u8]) -> Result<(), &'static str> {
    let mut offset = 0;
    let encoded_type = read_wire_string(blob, &mut offset)?;
    if encoded_type != key_type.as_bytes() {
        return Err("SSH public key type does not match its encoded key data");
    }

    match key_type {
        "ssh-ed25519" => {
            if read_wire_string(blob, &mut offset)?.len() != 32 {
                return Err("Ed25519 public key must contain exactly 32 bytes");
            }
        }
        "ecdsa-sha2-nistp256" => {
            if read_wire_string(blob, &mut offset)? != b"nistp256" {
                return Err("ECDSA key must use the NIST P-256 curve");
            }
            let point = read_wire_string(blob, &mut offset)?;
            if point.len() != 65 || point[0] != 4 {
                return Err("ECDSA public point is malformed");
            }
        }
        "sk-ssh-ed25519@openssh.com" => {
            if read_wire_string(blob, &mut offset)?.len() != 32 {
                return Err("Security-key Ed25519 public key must contain 32 bytes");
            }
            validate_application(read_wire_string(blob, &mut offset)?)?;
        }
        "sk-ecdsa-sha2-nistp256@openssh.com" => {
            if read_wire_string(blob, &mut offset)? != b"nistp256" {
                return Err("Security-key ECDSA key must use the NIST P-256 curve");
            }
            let point = read_wire_string(blob, &mut offset)?;
            if point.len() != 65 || point[0] != 4 {
                return Err("Security-key ECDSA public point is malformed");
            }
            validate_application(read_wire_string(blob, &mut offset)?)?;
        }
        "ssh-rsa" => {
            let exponent = positive_mpint(read_wire_string(blob, &mut offset)?)?;
            let modulus = positive_mpint(read_wire_string(blob, &mut offset)?)?;
            if exponent.len() > 8 {
                return Err("RSA public exponent is out of range");
            }
            let exponent_value = exponent.iter().try_fold(0_u64, |value, byte| {
                value.checked_mul(256)?.checked_add(u64::from(*byte))
            });
            if !matches!(exponent_value, Some(value) if value >= 3 && value % 2 == 1) {
                return Err("RSA public exponent must be an odd integer of at least 3");
            }

            let bits = (modulus.len() - 1) * 8
                + (8 - modulus[0].leading_zeros() as usize);
            if !(2048..=MAX_RSA_BITS).contains(&bits) {
                return Err("RSA modulus must be between 2048 and 8192 bits");
            }
        }
        _ => return Err("Unsupported SSH public key type"),
    }

    if offset != blob.len() {
        return Err("SSH public key contains unexpected trailing data");
    }
    Ok(())
}

pub(crate) fn fingerprint(
    key_type: &str,
    encoded_blob: &str,
) -> Result<String, &'static str> {
    if encoded_blob.len() > 12 * 1024 {
        return Err("SSH public key data is too large");
    }
    let blob = STANDARD
        .decode(encoded_blob)
        .or_else(|_| STANDARD_NO_PAD.decode(encoded_blob))
        .map_err(|_| "SSH public key data is not valid base64")?;
    if blob.len() > MAX_BLOB_BYTES {
        return Err("SSH public key data is too large");
    }
    validate_blob(key_type, &blob)?;
    Ok(format!(
        "SHA256:{}",
        STANDARD_NO_PAD.encode(Sha256::digest(blob))
    ))
}

#[cfg(test)]
mod tests {
    use super::fingerprint;
    use base64::{Engine as _, engine::general_purpose::STANDARD};

    fn wire_string(value: &[u8], output: &mut Vec<u8>) {
        output.extend_from_slice(&(value.len() as u32).to_be_bytes());
        output.extend_from_slice(value);
    }

    fn ed25519_blob(key_type: &str, public_key: &[u8]) -> Vec<u8> {
        let mut blob = Vec::new();
        wire_string(key_type.as_bytes(), &mut blob);
        wire_string(public_key, &mut blob);
        blob
    }

    #[test]
    fn accepts_well_formed_ed25519_and_security_keys() {
        let public_key = [7_u8; 32];
        let blob = ed25519_blob("ssh-ed25519", &public_key);
        assert!(fingerprint("ssh-ed25519", &STANDARD.encode(blob)).is_ok());

        let mut security_key = ed25519_blob("sk-ssh-ed25519@openssh.com", &public_key);
        wire_string(b"ssh:riotgit-test", &mut security_key);
        assert!(fingerprint(
            "sk-ssh-ed25519@openssh.com",
            &STANDARD.encode(security_key)
        )
        .is_ok());
    }

    #[test]
    fn rejects_truncated_mismatched_and_trailing_ed25519_data() {
        let short = ed25519_blob("ssh-ed25519", &[7_u8; 31]);
        assert!(fingerprint("ssh-ed25519", &STANDARD.encode(short)).is_err());

        let mismatched = ed25519_blob("ssh-rsa", &[7_u8; 32]);
        assert!(fingerprint("ssh-ed25519", &STANDARD.encode(mismatched)).is_err());

        let mut trailing = ed25519_blob("ssh-ed25519", &[7_u8; 32]);
        trailing.push(0);
        assert!(fingerprint("ssh-ed25519", &STANDARD.encode(trailing)).is_err());
    }

    #[test]
    fn rejects_invalid_base64_and_unsupported_key_types() {
        assert!(fingerprint("ssh-ed25519", "%%%").is_err());
        assert!(fingerprint("ssh-dss", "AAAA").is_err());
        assert!(fingerprint("ssh-ed25519", &STANDARD.encode([0_u8; 4])).is_err());
    }

    #[test]
    fn validates_ecdsa_curve_and_point_shape() {
        let mut valid = Vec::new();
        wire_string(b"ecdsa-sha2-nistp256", &mut valid);
        wire_string(b"nistp256", &mut valid);
        let mut point = vec![4_u8];
        point.extend_from_slice(&[9_u8; 64]);
        wire_string(&point, &mut valid);
        assert!(fingerprint("ecdsa-sha2-nistp256", &STANDARD.encode(valid)).is_ok());

        let mut wrong_curve = Vec::new();
        wire_string(b"ecdsa-sha2-nistp256", &mut wrong_curve);
        wire_string(b"nistp384", &mut wrong_curve);
        wire_string(&point, &mut wrong_curve);
        assert!(fingerprint(
            "ecdsa-sha2-nistp256",
            &STANDARD.encode(wrong_curve)
        )
        .is_err());

        let mut bad_point = Vec::new();
        wire_string(b"ecdsa-sha2-nistp256", &mut bad_point);
        wire_string(b"nistp256", &mut bad_point);
        wire_string(&[4_u8; 65], &mut bad_point);
        assert!(fingerprint(
            "ecdsa-sha2-nistp256",
            &STANDARD.encode(bad_point)
        )
        .is_err());
    }

    #[test]
    fn validates_security_key_application_and_rsa_strength() {
        let mut missing_application = ed25519_blob("sk-ssh-ed25519@openssh.com", &[3_u8; 32]);
        wire_string(b"", &mut missing_application);
        assert!(fingerprint(
            "sk-ssh-ed25519@openssh.com",
            &STANDARD.encode(missing_application)
        )
        .is_err());

        let mut weak_rsa = Vec::new();
        wire_string(b"ssh-rsa", &mut weak_rsa);
        wire_string(&[1, 0, 1], &mut weak_rsa);
        let mut weak_modulus = vec![0; 129];
        weak_modulus[1] = 0x80;
        wire_string(&weak_modulus, &mut weak_rsa);
        assert!(fingerprint("ssh-rsa", &STANDARD.encode(weak_rsa)).is_err());

        let mut rsa = Vec::new();
        wire_string(b"ssh-rsa", &mut rsa);
        wire_string(&[1, 0, 1], &mut rsa);
        let mut modulus = vec![0_u8; 257];
        modulus[1] = 0x80;
        wire_string(&modulus, &mut rsa);
        assert!(fingerprint("ssh-rsa", &STANDARD.encode(rsa)).is_ok());
    }
}
