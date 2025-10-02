FROM openjdk:21

WORKDIR /workspace

COPY judge.sh /judge.sh
RUN chmod +x /judge.sh

ENTRYPOINT ["/judge.sh"]
