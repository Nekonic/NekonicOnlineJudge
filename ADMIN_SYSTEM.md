# 관리자 및 그룹 관리 시스템

## 개요
이 시스템은 관리자와 일반 사용자를 구분하여 그룹 생성 및 가입을 관리합니다.

## 주요 기능

### 1. 사용자 역할 시스템
- **관리자 (admin)**: 모든 권한 보유
- **일반 사용자 (user)**: 기본 기능 사용

### 2. 그룹 관리 기능

#### 일반 사용자
- 그룹 생성 요청 (관리자 승인 필요)
- 그룹 가입 요청 (관리자 승인 필요)
- 내 그룹 목록 확인
- 그룹 상세 정보 조회

#### 관리자
- 그룹 생성 (즉시 승인)
- 그룹 승인/거부
- 가입 요청 승인/거부
- 그룹에 사용자 직접 추가
- 사용자를 관리자로 승격
- 대회 관리

## 사용 방법

### 첫 관리자 설정

1. 서버 실행 및 마이그레이션:
```bash
cargo run
```

2. 첫 번째 사용자를 관리자로 수동 설정:
```bash
sqlite3 database.sqlite
```

```sql
-- 첫 번째 사용자를 관리자로 변경
UPDATE users SET role = 'admin' WHERE id = 1;
```

3. 또는 다른 관리자가 있다면 `/admin/users/{user_id}/promote` API 사용

### 관리자 기능

#### 관리자 대시보드
- URL: `/admin`
- 대기 중인 그룹 승인 요청 확인
- 대기 중인 가입 요청 확인
- 최근 관리자 액션 로그 확인

#### 그룹 승인 관리
- URL: `/admin/organizations/pending`
- 사용자가 생성한 그룹 요청을 승인/거부

#### 가입 요청 관리
- URL: `/admin/join-requests/pending`
- 사용자의 그룹 가입 요청을 승인/거부

#### 관리자가 그룹 생성
```bash
POST /admin/organizations/create
{
  "name": "그룹명",
  "type": "school|company|study|other",
  "description": "설명"
}
```
- 즉시 승인된 상태로 생성됨

#### 그룹에 사용자 추가
```bash
POST /admin/organizations/{org_id}/members/add
{
  "user_id": 123,
  "role": "MEMBER|ADMIN"
}
```

### 일반 사용자 기능

#### 그룹 생성 요청
- URL: `/organizations`
- "그룹 생성" 버튼 클릭
- 관리자 승인 대기

#### 그룹 가입 요청
- URL: `/organizations/{id}`
- "가입 요청" 버튼 클릭
- 관리자 승인 대기

#### 내 그룹 확인
- URL: `/organizations/my`
- 가입된 그룹 및 승인 대기 중인 그룹 확인

## 데이터베이스 스키마

### users 테이블
- `role`: 'admin' 또는 'user'

### organizations 테이블
- `status`: 'pending', 'approved', 'rejected'
- `created_by`: 생성자 user_id
- `approved_by`: 승인한 관리자 user_id

### organization_join_requests 테이블
- `status`: 'pending', 'approved', 'rejected'
- `reviewed_by`: 검토한 관리자 user_id

### admin_actions 테이블
- 모든 관리자 액션 로그 기록

## API 엔드포인트

### 공개 라우트
- `GET /organizations` - 그룹 목록
- `GET /organizations/{id}` - 그룹 상세

### 인증 필요 라우트
- `POST /organizations/create` - 그룹 생성 요청
- `POST /organizations/{id}/join` - 가입 요청
- `GET /organizations/my` - 내 그룹

### 관리자 전용 라우트
- `GET /admin` - 관리자 대시보드
- `GET /admin/organizations/pending` - 승인 대기 그룹
- `POST /admin/organizations/{id}/review` - 그룹 승인/거부
- `POST /admin/organizations/create` - 관리자가 그룹 생성
- `POST /admin/organizations/{id}/members/add` - 멤버 추가
- `GET /admin/join-requests/pending` - 가입 요청 목록
- `POST /admin/join-requests/{id}/review` - 가입 요청 승인/거부
- `POST /admin/users/{id}/promote` - 관리자 승격

## 보안 기능

1. **미들웨어 기반 권한 검증**
   - `require_admin`: 관리자만 접근 가능
   - `require_auth`: 로그인 사용자만 접근 가능

2. **감사 로그 (Audit Log)**
   - 모든 관리자 액션이 `admin_actions` 테이블에 기록됨

3. **상태 관리**
   - 그룹과 가입 요청의 상태를 명확하게 관리
   - pending → approved/rejected 흐름

## 예제 워크플로우

### 사용자가 그룹 생성하는 경우
1. 사용자가 `/organizations`에서 "그룹 생성" 클릭
2. 그룹 정보 입력 후 제출 → `status: 'pending'`
3. 관리자가 `/admin/organizations/pending`에서 확인
4. 승인 → `status: 'approved'`, 거부 → `status: 'rejected'`

### 사용자가 그룹에 가입하는 경우
1. 사용자가 `/organizations/{id}`에서 "가입 요청" 클릭
2. 가입 메시지 입력 후 제출
3. 관리자가 `/admin/join-requests/pending`에서 확인
4. 승인 → `user_organizations`에 추가, 거부 → 거부됨

### 관리자가 직접 그룹 생성 및 멤버 추가
1. 관리자가 `/admin`에서 그룹 생성
2. 즉시 `status: 'approved'`로 생성
3. `/admin/organizations/{id}/members/add`로 사용자 추가
4. 승인 절차 없이 즉시 멤버로 등록

## 마이그레이션

새로운 마이그레이션 파일이 추가되었습니다:
- `migrations/002_add_roles_and_approvals.sql`

서버 시작 시 자동으로 적용됩니다.

## 주의사항

1. **첫 관리자 설정 필수**: 시스템 사용 전 최소 1명의 관리자 필요
2. **데이터베이스 백업**: 마이그레이션 전 데이터베이스 백업 권장
3. **권한 확인**: 관리자 권한이 필요한 작업은 반드시 로그인 상태 확인

