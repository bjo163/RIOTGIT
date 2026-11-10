import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "R!OTGIT — The Underground Code Network",
  description: "Break the rules. Own your code.",
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en" data-theme="riot-dark">
      <body>{children}</body>
    </html>
  );
}
