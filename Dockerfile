# ============================
# Stage 1: Builder
# ============================
FROM python:3.12-slim AS builder

WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

RUN python -m venv /opt/venv
ENV PATH="/opt/venv/bin:$PATH"

COPY pyproject.toml ./
RUN pip install --no-cache-dir --upgrade pip wheel && \
    pip install --no-cache-dir -e ".[all]"

# ============================
# Stage 2: Runtime
# ============================
FROM python:3.12-slim AS runtime

WORKDIR /app

# Runtime deps
RUN apt-get update && apt-get install -y --no-install-recommends \
    tesseract-ocr \
    tesseract-ocr-eng \
    curl \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /opt/venv /opt/venv
ENV PATH="/opt/venv/bin:$PATH"
ENV PYTHONUNBUFFERED=1

# Data directories
RUN mkdir -p /data /home/airos/.ai_research_os

COPY . .

RUN useradd -m -u 1000 airos && \
    chown -R airos:airos /app /data /home/airos
USER airos

EXPOSE 8501

ENTRYPOINT ["uvicorn"]
CMD ["web.app:app", "--host", "0.0.0.0", "--port", "8501"]
