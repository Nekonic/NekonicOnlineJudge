-- 사용자 테이블
CREATE TABLE users (
                       id INTEGER PRIMARY KEY AUTOINCREMENT,
                       username TEXT NOT NULL UNIQUE,
                       password_hash TEXT NOT NULL,
                       created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- 제출 테이블
CREATE TABLE submissions (
                             id INTEGER PRIMARY KEY AUTOINCREMENT,
                             user_id INTEGER NOT NULL,
                             problem_id INTEGER NOT NULL,
                             language VARCHAR(20) NOT NULL,
                             source_code TEXT NOT NULL,

    -- 채점 결과
                             status VARCHAR(30) DEFAULT 'PENDING', -- PENDING, COMPILING, JUDGING, ACCEPTED, WRONG_ANSWER, TIME_LIMIT_EXCEEDED, MEMORY_LIMIT_EXCEEDED, RUNTIME_ERROR, COMPILATION_ERROR, SYSTEM_ERROR

    -- 점수 및 성능
                             score INTEGER DEFAULT 0,              -- 부분 점수 (0-100)
                             execution_time INTEGER,               -- 실행 시간 (ms)
                             memory_usage INTEGER,                 -- 메모리 사용량 (KB)

    -- 에러 및 메시지
                             compile_message TEXT,                 -- 컴파일 에러 메시지
                             runtime_error_type VARCHAR(50),       -- SEGFAULT, OUT_OF_BOUNDS, STACK_OVERFLOW, etc.
                             runtime_error_message TEXT,           -- 런타임 에러 상세 메시지

    -- 테스트 케이스별 결과
                             total_testcases INTEGER DEFAULT 0,    -- 총 테스트 케이스 수
                             passed_testcases INTEGER DEFAULT 0,   -- 통과한 테스트 케이스 수

                             created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                             judged_at DATETIME,                   -- 채점 완료 시간

                             FOREIGN KEY (user_id) REFERENCES users(id)
);

-- 테스트 케이스별 상세 결과 테이블
CREATE TABLE testcase_results (
                                  id INTEGER PRIMARY KEY AUTOINCREMENT,
                                  submission_id INTEGER NOT NULL,
                                  testcase_number INTEGER NOT NULL,
                                  status VARCHAR(30) NOT NULL,          -- ACCEPTED, WRONG_ANSWER, TIME_LIMIT_EXCEEDED, RUNTIME_ERROR, etc.
                                  execution_time INTEGER,               -- 해당 테스트케이스 실행 시간 (ms)
                                  memory_usage INTEGER,                 -- 해당 테스트케이스 메모리 사용량 (KB)
                                  error_message TEXT,                   -- 에러 메시지 (있는 경우)
                                  expected_output TEXT,                 -- 예상 출력 (틀린 경우에만)
                                  actual_output TEXT,                   -- 실제 출력 (틀린 경우에만)
                                  created_at DATETIME DEFAULT CURRENT_TIMESTAMP,

                                  FOREIGN KEY (submission_id) REFERENCES submissions(id) ON DELETE CASCADE
);

-- 컴파일 에러 상세 정보 테이블 (옵션)
CREATE TABLE compile_errors (
                                id INTEGER PRIMARY KEY AUTOINCREMENT,
                                submission_id INTEGER NOT NULL,
                                line_number INTEGER,
                                column_number INTEGER,
                                error_type VARCHAR(50),               -- SYNTAX_ERROR, TYPE_ERROR, etc.
                                error_message TEXT NOT NULL,
                                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,

                                FOREIGN KEY (submission_id) REFERENCES submissions(id) ON DELETE CASCADE
);

-- 인덱스 추가
CREATE INDEX idx_submissions_user_id ON submissions(user_id);
CREATE INDEX idx_submissions_problem_id ON submissions(problem_id);
CREATE INDEX idx_submissions_status ON submissions(status);
CREATE INDEX idx_submissions_created_at ON submissions(created_at DESC);
CREATE INDEX idx_testcase_results_submission_id ON testcase_results(submission_id);
CREATE INDEX idx_compile_errors_submission_id ON compile_errors(submission_id);

-- 채점 통계를 위한 뷰
CREATE VIEW submission_stats AS
SELECT
    problem_id,
    COUNT(*) as total_submissions,
    COUNT(CASE WHEN status = 'ACCEPTED' THEN 1 END) as accepted_submissions,
    ROUND(
            CAST(COUNT(CASE WHEN status = 'ACCEPTED' THEN 1 END) AS FLOAT) /
            CAST(COUNT(*) AS FLOAT) * 100, 3
    ) as acceptance_rate,
    AVG(CASE WHEN status = 'ACCEPTED' THEN execution_time END) as avg_execution_time,
    AVG(CASE WHEN status = 'ACCEPTED' THEN memory_usage END) as avg_memory_usage
FROM submissions
WHERE status != 'PENDING'
GROUP BY problem_id;
