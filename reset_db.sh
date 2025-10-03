#!/bin/bash

# 개발용 데이터베이스 리셋 스크립트
# 주의: 모든 데이터가 삭제됩니다!

set -e

echo "🗑️  데이터베이스 파일 삭제 중..."
rm -f database.sqlite database.sqlite-shm database.sqlite-wal

echo "📦 데이터베이스 생성 및 마이그레이션 적용 중..."
sqlx database create
sqlx migrate run

echo "✅ 데이터베이스 리셋 완료!"

# 테스트 계정 생성
echo ""
echo "👤 테스트 계정 생성 중..."
echo "   (서버를 짧게 실행하여 회원가입 API로 계정 생성)"

# 백그라운드에서 서버 실행
cargo run > /tmp/server.log 2>&1 &
SERVER_PID=$!

# 서버가 시작될 때까지 대기
echo "   서버 시작 대기 중..."
sleep 5

# 회원가입 API로 계정 생성
echo "   admin 계정 생성 중..."
curl -s -X POST http://localhost:3000/register \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "username=admin&password=admin123" > /dev/null 2>&1 || true

echo "   user1 계정 생성 중..."
curl -s -X POST http://localhost:3000/register \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "username=user1&password=admin123" > /dev/null 2>&1 || true

# 서버 종료
kill $SERVER_PID 2>/dev/null || true
wait $SERVER_PID 2>/dev/null || true

# 관리자 권한 부여
echo "   관리자 권한 부여 중..."
sqlite3 database.sqlite "UPDATE users SET role = 'admin' WHERE username = 'admin';"

# 샘플 그룹 생성
echo "   샘플 그룹 생성 중..."
sqlite3 database.sqlite <<EOF
INSERT INTO organizations (name, type, description, status, created_by, approved_by, approved_at)
VALUES ('테스트 학교', 'school', '테스트용 학교 그룹', 'approved', 1, 1, CURRENT_TIMESTAMP);

INSERT INTO user_organizations (user_id, organization_id, role, added_by)
VALUES (2, 1, 'MEMBER', 1);
EOF

echo "✅ 테스트 계정 생성 완료!"
echo ""
echo "📋 테스트 계정 정보:"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "관리자 계정:"
echo "  Username: admin"
echo "  Password: admin123"
echo ""
echo "일반 사용자:"
echo "  Username: user1"
echo "  Password: admin123"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "🚀 서버 실행: cargo run"
echo "🔐 관리자 대시보드: http://localhost:3000/admin"
