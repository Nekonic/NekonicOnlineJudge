#!/bin/bash

# 개발용 데이터베이스 리셋 스크립트
# 주의: 모든 데이터가 삭제됩니다!

set -e

echo "🗑️  데이터베이스 파일 삭제 중..."
rm -f database.sqlite database.sqlite-shm database.sqlite-wal

echo "📦 데이터베이스 생성 중..."
sqlx database create

echo "🔧 마이그레이션 실행 중..."
sqlx migrate run

echo "✅ 데이터베이스 리셋 완료!"

