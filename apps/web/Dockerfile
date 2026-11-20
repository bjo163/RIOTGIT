FROM node:26.13.1-bookworm-slim AS dependencies
WORKDIR /app
COPY apps/web/package.json apps/web/package-lock.json apps/web/.npm-version ./
RUN test "$(npm --version)" = "$(cat .npm-version)" && npm ci --no-audit --no-fund

FROM node:26.13.1-bookworm-slim AS builder
WORKDIR /app
ENV NEXT_TELEMETRY_DISABLED=1
COPY --from=dependencies /app/node_modules ./node_modules
COPY apps/web/ ./
RUN npm run build

FROM node:26.13.1-bookworm-slim AS runtime
WORKDIR /app
ENV NODE_ENV=production \
    NEXT_TELEMETRY_DISABLED=1 \
    PORT=3000 \
    HOSTNAME=0.0.0.0
COPY --from=builder --chown=node:node /app/.next/standalone ./
COPY --from=builder --chown=node:node /app/.next/static ./.next/static
USER node
EXPOSE 3000
CMD ["node", "server.js"]
