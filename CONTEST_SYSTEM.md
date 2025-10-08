# 대회 시스템 구현 완료

## 구현된 기능

### Phase 1: 기본 CRUD
✅ **대회 생성**
- 로그인한 사용자는 누구나 대회를 만들 수 있습니다
- 관리자가 만든 대회는 즉시 승인되며, 일반 사용자는 승인 대기 상태
- 대회 정보: 제목, 설명, 시작/종료 시간, 유형(ICPC/IOI/CTF/PRACTICE)

✅ **대회 목록 조회**
- 진행 중, 예정, 종료된 대회를 구분하여 표시
- 실시간으로 대회 상태 판단 (현재 시간 기준)

✅ **대회 상세 페이지**
- 대회 정보, 참가자 수, 문제 목록 표시
- 대회 생성자/관리자에게는 관리 버튼 표시
- 참가 신청 기능

### Phase 2: 문제 관리
✅ **대회 관리 페이지** (`/contests/{id}/manage`)
- 대회 생성자와 관리자만 접근 가능
- 문제 추가/삭제 기능
- 각 문제의 배점과 순서 설정

✅ **대회 문제 접근**
- 대회 참가자만 대회 중에 문제 접근 가능
- 대회 시작 전이나 종료 후에는 접근 제한
- 일반 문제 페이지와 연동

✅ **대회 제출 시스템**
- 대회 중에만 제출 가능
- 제출 시 contest_id가 자동으로 기록됨
- 대회 시간 외 제출은 거부

### Phase 3: 순위 시스템
✅ **ICPC 스타일 순위 계산**
- 맞춘 문제 수가 많을수록 상위
- 같은 문제 수면 패널티 타임이 적을수록 상위
- 패널티 = 문제를 푼 시간(분) + 틀린 횟수 × 20분

✅ **CTF 스타일 순위 계산**
- 각 문제의 배점을 합산
- First Blood 보너스 (추후 확장 가능)

✅ **자동 순위 업데이트**
- 제출이 채점 완료되면 자동으로 순위 업데이트
- 대회 참가자의 점수와 패널티 자동 계산

✅ **순위표 페이지** (`/contests/{id}/standings`)
- 실시간 순위 표시
- 1등/2등/3등 메달 표시
- 현재 사용자 하이라이트

## 엔드포인트

### 공개 엔드포인트
- `GET /contests` - 대회 목록
- `GET /contests/create` - 대회 생성 페이지
- `POST /contests/create` - 대회 생성 액션
- `GET /contests/{id}` - 대회 상세
- `POST /contests/{id}/register` - 대회 참가 신청
- `GET /contests/{id}/standings` - 순위표

### 인증 필요 엔드포인트
- `GET /contests/{id}/manage` - 대회 관리 페이지
- `POST /contests/{id}/problems/add` - 문제 추가
- `POST /contests/{contest_id}/problems/{problem_id}/remove` - 문제 삭제
- `GET /contests/{contest_id}/problems/{problem_id}` - 대회 문제 페이지
- `POST /contests/{contest_id}/problems/{problem_id}/submit` - 대회 문제 제출

## 데이터베이스 구조

### contests 테이블
- 대회 기본 정보 (제목, 설명, 시간, 유형, 상태)
- contest_type: ICPC, IOI, CTF, PRACTICE

### contest_problems 테이블
- 대회와 문제의 매핑
- 각 문제의 배점(points)과 순서(problem_order)

### contest_participants 테이블
- 대회 참가자 정보
- total_score: 맞춘 문제 수 또는 총 점수
- penalty_time: ICPC 스타일 패널티

### submissions 테이블
- contest_id 컬럼으로 대회 제출 구분
- 일반 제출과 대회 제출 구분 가능

## 사용 흐름

### 1. 대회 생성 및 설정
1. `/contests/create`에서 대회 생성
2. `/contests/{id}/manage`에서 문제 추가
3. 관리자는 승인 (일반 사용자가 만든 경우)

### 2. 대회 참가
1. `/contests`에서 대회 목록 확인
2. `/contests/{id}`에서 대회 정보 확인
3. "참가 신청" 버튼 클릭

### 3. 대회 진행
1. 대회 시작 시간이 되면 "진행중" 상태로 변경
2. 참가자는 문제 목록에서 문제 클릭
3. 문제 풀이 및 제출
4. 채점 완료 시 자동으로 순위 업데이트

### 4. 순위 확인
1. `/contests/{id}/standings`에서 실시간 순위 확인
2. 자신의 순위는 파란색으로 하이라이트
3. 상위 3명은 메달 표시

## 채점 방식

### ICPC/IOI/PRACTICE
- 각 문제별로 ACCEPTED 여부 확인
- 첫 ACCEPTED까지의 시간 계산
- 틀린 제출마다 20분 패널티 추가
- 정렬: solved DESC, penalty ASC

### CTF
- 각 문제의 배점을 합산
- ACCEPTED된 문제만 점수 획득
- 정렬: total_score DESC

## 다음 단계 확장 가능 기능

### 고급 기능
- [ ] Virtual participation (가상 참가)
- [ ] Team contest (팀 대회)
- [ ] Editorial (풀이 공개)
- [ ] First Blood 보너스 (CTF)
- [ ] Scoreboard Freeze (마지막 1시간 순위 고정)
- [ ] 대회별 제출 내역
- [ ] 문제별 통계 (정답률, 평균 시도 횟수)

### UI 개선
- [ ] 실시간 순위 업데이트 (WebSocket)
- [ ] 문제별 풀이 상태 표시 (O, X, ?)
- [ ] 대회 타이머
- [ ] 알림 시스템

### 관리 기능
- [ ] 대회 수정
- [ ] 참가자 관리 (강제 추가/제거)
- [ ] 대회 복제
- [ ] 통계 대시보드

## 파일 구조

```
src/
  handlers/
    contests.rs          # 대회 관련 모든 핸들러
  contest_scoring.rs     # 순위 계산 로직
  models.rs              # Contest 관련 모델들
  router.rs              # 라우트 설정

templates/
  contests_list.html     # 대회 목록
  contest_detail.html    # 대회 상세
  contest_create.html    # 대회 생성
  contest_manage.html    # 대회 관리
  contest_standings.html # 순위표
  contest_problem.html   # 대회 문제
```

## 테스트 방법

1. 데이터베이스 초기화
```bash
bash reset_db.sh
```

2. 서버 실행
```bash
cargo run
```

3. 테스트 시나리오
   - admin/admin123으로 로그인
   - 대회 생성 (예: 2025-01-05 10:00 ~ 2025-01-05 12:00)
   - 문제 추가 (예: 1001, 1002)
   - user1/admin123으로 로그인
   - 대회 참가 신청
   - 문제 풀이 및 제출
   - 순위표에서 점수 확인

## 주의사항

- 대회 시작 시간은 ISO 8601 형식 또는 SQLite datetime 형식
- 대회 중에만 문제 제출 가능
- 채점 완료 시 자동으로 순위 업데이트
- 대회 생성자와 관리자만 대회 관리 가능

