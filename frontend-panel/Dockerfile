FROM node:20-alpine
WORKDIR /app

COPY frontend-panel/package.json ./
COPY frontend-panel/package-lock.json ./
RUN npm ci

COPY frontend-panel .

EXPOSE 5173
CMD ["npm", "run", "dev"]
