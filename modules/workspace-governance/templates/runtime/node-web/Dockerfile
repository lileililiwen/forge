FROM node:22-bookworm-slim AS build
WORKDIR /app
COPY package*.json ./
RUN npm ci
COPY . .
RUN {{build_command}}

FROM node:22-bookworm-slim AS runtime
WORKDIR /app
ENV NODE_ENV=production
RUN useradd --create-home --uid 10001 appuser
COPY package*.json ./
RUN npm ci --omit=dev && npm cache clean --force
COPY --from=build /app/{{output_dir}} ./{{output_dir}}
COPY --from=build /app/package.json ./package.json
USER appuser
EXPOSE 3000
CMD ["sh", "-c", "{{start_command}}"]
