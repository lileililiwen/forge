FROM python:3.12-slim AS runtime
ENV PYTHONDONTWRITEBYTECODE=1 \
    PYTHONUNBUFFERED=1 \
    PIP_NO_CACHE_DIR=1
WORKDIR /app
RUN apt-get update \
    && apt-get install -y --no-install-recommends curl \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --create-home --uid 10001 appuser
COPY requirements*.txt ./
RUN if [ -f requirements.txt ]; then pip install -r requirements.txt; fi
COPY . .
USER appuser
EXPOSE 8000
CMD ["sh", "-c", "uvicorn {{entrypoint}} --host 0.0.0.0 --port 8000"]
