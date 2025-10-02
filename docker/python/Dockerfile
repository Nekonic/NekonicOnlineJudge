FROM python:3.12-slim

WORKDIR /workspace

COPY judge.sh /judge.sh
RUN chmod +x /judge.sh

ENTRYPOINT ["/judge.sh"]
