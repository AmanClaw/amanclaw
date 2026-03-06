FROM node:22-slim

RUN groupadd --gid 1000 bridge && \
    useradd --uid 1000 --gid 1000 --create-home bridge

WORKDIR /app

COPY package.json package-lock.json* ./
RUN npm ci --omit=dev && npm cache clean --force

COPY index.js ./

RUN mkdir -p /data/auth_state && chown -R bridge:bridge /app /data

USER bridge

CMD ["node", "index.js"]
