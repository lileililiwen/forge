FROM python:3.12-slim
ENV PYTHONDONTWRITEBYTECODE=1 \
    PYTHONUNBUFFERED=1 \
    PIP_NO_CACHE_DIR=1
WORKDIR /app
RUN apt-get update \
    && apt-get install -y --no-install-recommends postgresql-client \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --create-home --uid 10001 appuser
COPY requirements*.txt ./
RUN if [ -f requirements.txt ]; then pip install -r requirements.txt; fi
COPY . .
USER appuser
CMD ["sh", "-c", "{{worker_command}}"]
