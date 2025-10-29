-- ============================================
-- 사용자 및 인증
-- ============================================
CREATE TABLE users (
                       id INTEGER PRIMARY KEY AUTOINCREMENT,
                       username TEXT NOT NULL UNIQUE,
                       password_hash TEXT NOT NULL,
                       user_type VARCHAR(20) DEFAULT 'individual' NOT NULL,
                       role VARCHAR(20) DEFAULT 'user' NOT NULL,
                       created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- ============================================
-- 제출 및 채점
-- ============================================
CREATE TABLE submissions (
                             id INTEGER PRIMARY KEY AUTOINCREMENT,
                             user_id INTEGER NOT NULL,
                             problem_id INTEGER NOT NULL,
                             contest_id INTEGER,
                             language VARCHAR(20) NOT NULL,
                             source_code TEXT NOT NULL,

    -- 채점 결과
                             status VARCHAR(30) DEFAULT 'PENDING',

    -- 점수 및 성능
                             score INTEGER DEFAULT 0,
                             execution_time INTEGER,
                             memory_usage INTEGER,

    -- 에러 및 메시지
                             compile_message TEXT,
                             runtime_error_type VARCHAR(50),
                             runtime_error_message TEXT,

    -- 테스트 케이스
                             total_testcases INTEGER DEFAULT 0,
                             passed_testcases INTEGER DEFAULT 0,

                             created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                             judged_at DATETIME,

                             FOREIGN KEY (user_id) REFERENCES users(id),
                             FOREIGN KEY (contest_id) REFERENCES contests(id)
);

CREATE TABLE testcase_results (
                                  id INTEGER PRIMARY KEY AUTOINCREMENT,
                                  submission_id INTEGER NOT NULL,
                                  testcase_number INTEGER NOT NULL,
                                  status VARCHAR(30) NOT NULL,
                                  execution_time INTEGER,
                                  memory_usage INTEGER,
                                  error_message TEXT,
                                  expected_output TEXT,
                                  actual_output TEXT,
                                  created_at DATETIME DEFAULT CURRENT_TIMESTAMP,

                                  FOREIGN KEY (submission_id) REFERENCES submissions(id) ON DELETE CASCADE
);

CREATE TABLE compile_errors (
                                id INTEGER PRIMARY KEY AUTOINCREMENT,
                                submission_id INTEGER NOT NULL,
                                line_number INTEGER,
                                column_number INTEGER,
                                error_type VARCHAR(50),
                                error_message TEXT NOT NULL,
                                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,

                                FOREIGN KEY (submission_id) REFERENCES submissions(id) ON DELETE CASCADE
);

-- ============================================
-- 조직/그룹
-- ============================================
CREATE TABLE organizations (
                               id INTEGER PRIMARY KEY AUTOINCREMENT,
                               name TEXT NOT NULL,
                               type VARCHAR(20) NOT NULL,
                               description TEXT,
                               status VARCHAR(20) DEFAULT 'pending' NOT NULL,
                               created_by INTEGER,
                               approved_by INTEGER,
                               approved_at DATETIME,
                               created_at DATETIME DEFAULT CURRENT_TIMESTAMP,

                               FOREIGN KEY (created_by) REFERENCES users(id),
                               FOREIGN KEY (approved_by) REFERENCES users(id)
);

CREATE TABLE user_organizations (
                                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                                    user_id INTEGER NOT NULL,
                                    organization_id INTEGER NOT NULL,
                                    role VARCHAR(20) DEFAULT 'MEMBER',
                                    added_by INTEGER,
                                    status VARCHAR(20) DEFAULT 'active' NOT NULL,
                                    joined_at DATETIME DEFAULT CURRENT_TIMESTAMP,

                                    FOREIGN KEY (user_id) REFERENCES users(id),
                                    FOREIGN KEY (organization_id) REFERENCES organizations(id) ON DELETE CASCADE,
                                    FOREIGN KEY (added_by) REFERENCES users(id),
                                    UNIQUE(user_id, organization_id)
);

-- 그룹 가입 요청 테이블
CREATE TABLE organization_join_requests (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    organization_id INTEGER NOT NULL,
    user_id INTEGER NOT NULL,
    status VARCHAR(20) DEFAULT 'pending' NOT NULL,
    message TEXT,
    requested_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    reviewed_by INTEGER,
    reviewed_at DATETIME,

    FOREIGN KEY (organization_id) REFERENCES organizations(id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (reviewed_by) REFERENCES users(id),
    UNIQUE(organization_id, user_id, status)
);

-- ============================================
-- 대회 시스템
-- ============================================
CREATE TABLE contests (
                          id INTEGER PRIMARY KEY AUTOINCREMENT,
                          title TEXT NOT NULL,
                          description TEXT,
                          start_time DATETIME NOT NULL,
                          end_time DATETIME NOT NULL,
                          contest_type VARCHAR(20) DEFAULT 'RATED',
                          is_public BOOLEAN DEFAULT 1,
                          max_participants INTEGER,
                          status VARCHAR(20) DEFAULT 'draft' NOT NULL,
                          requires_approval BOOLEAN DEFAULT 0,
                          approved_by INTEGER,
                          approved_at DATETIME,
                          created_by INTEGER NOT NULL,
                          created_at DATETIME DEFAULT CURRENT_TIMESTAMP,

                          FOREIGN KEY (created_by) REFERENCES users(id),
                          FOREIGN KEY (approved_by) REFERENCES users(id)
);

CREATE TABLE contest_problems (
                                  id INTEGER PRIMARY KEY AUTOINCREMENT,
                                  contest_id INTEGER NOT NULL,
                                  problem_id INTEGER NOT NULL,
                                  points INTEGER NOT NULL DEFAULT 100,
                                  problem_order INTEGER NOT NULL,

                                  FOREIGN KEY (contest_id) REFERENCES contests(id),
                                  UNIQUE(contest_id, problem_id)
);

CREATE TABLE contest_participants (
                                      id INTEGER PRIMARY KEY AUTOINCREMENT,
                                      contest_id INTEGER NOT NULL,
                                      user_id INTEGER NOT NULL,
                                      total_score INTEGER DEFAULT 0,
                                      penalty_time INTEGER DEFAULT 0,
                                      joined_at DATETIME DEFAULT CURRENT_TIMESTAMP,

                                      FOREIGN KEY (contest_id) REFERENCES contests(id),
                                      FOREIGN KEY (user_id) REFERENCES users(id),
                                      UNIQUE(contest_id, user_id)
);

-- ============================================
-- 관리자 액션 로그
-- ============================================
CREATE TABLE admin_actions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    admin_id INTEGER NOT NULL,
    action_type VARCHAR(50) NOT NULL,
    target_type VARCHAR(50) NOT NULL,
    target_id INTEGER NOT NULL,
    details TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,

    FOREIGN KEY (admin_id) REFERENCES users(id)
);

-- ============================================
-- 사용자 통계 및 랭킹
-- ============================================
CREATE TABLE user_stats (
                            user_id INTEGER PRIMARY KEY,
                            total_solved INTEGER DEFAULT 0,
                            total_submissions INTEGER DEFAULT 0,
                            rating INTEGER DEFAULT 1500,
                            max_streak INTEGER DEFAULT 0,
                            current_streak INTEGER DEFAULT 0,
                            last_solved_date DATE,

                            FOREIGN KEY (user_id) REFERENCES users(id)
);

CREATE TABLE rating_history (
                                id INTEGER PRIMARY KEY AUTOINCREMENT,
                                user_id INTEGER NOT NULL,
                                contest_id INTEGER NOT NULL,
                                old_rating INTEGER NOT NULL,
                                new_rating INTEGER NOT NULL,
                                rank INTEGER NOT NULL,
                                recorded_at DATETIME DEFAULT CURRENT_TIMESTAMP,

                                FOREIGN KEY (user_id) REFERENCES users(id),
                                FOREIGN KEY (contest_id) REFERENCES contests(id)
);

CREATE TABLE ranking_cache (
                               user_id INTEGER PRIMARY KEY,
                               global_rank INTEGER,
                               rating_rank INTEGER,
                               last_updated DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- ============================================
-- 인덱스
-- ============================================
CREATE INDEX idx_users_role ON users(role);
CREATE INDEX idx_submissions_user_id ON submissions(user_id);
CREATE INDEX idx_submissions_problem_id ON submissions(problem_id);
CREATE INDEX idx_submissions_contest_id ON submissions(contest_id);
CREATE INDEX idx_submissions_status ON submissions(status);
CREATE INDEX idx_submissions_created_at ON submissions(created_at DESC);
CREATE INDEX idx_testcase_results_submission_id ON testcase_results(submission_id);
CREATE INDEX idx_compile_errors_submission_id ON compile_errors(submission_id);
CREATE INDEX idx_contest_participants_contest_id ON contest_participants(contest_id);
CREATE INDEX idx_contest_participants_user_id ON contest_participants(user_id);
CREATE INDEX idx_contests_status ON contests(status);
CREATE INDEX idx_contests_created_by ON contests(created_by);
CREATE INDEX idx_user_organizations_user_id ON user_organizations(user_id);
CREATE INDEX idx_user_organizations_org_id ON user_organizations(organization_id);
CREATE INDEX idx_organizations_status ON organizations(status);
CREATE INDEX idx_organizations_created_by ON organizations(created_by);
CREATE INDEX idx_organization_join_requests_status ON organization_join_requests(status);
CREATE INDEX idx_organization_join_requests_org_id ON organization_join_requests(organization_id);
CREATE INDEX idx_organization_join_requests_user_id ON organization_join_requests(user_id);
CREATE INDEX idx_admin_actions_admin_id ON admin_actions(admin_id);
CREATE INDEX idx_admin_actions_created_at ON admin_actions(created_at DESC);

-- ============================================
-- 게시판 시스템
-- ============================================
CREATE TABLE boards (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    board_type VARCHAR(20) NOT NULL, -- 'announcement', 'free', 'qna', 'organization'
    organization_id INTEGER,
    description TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,

    FOREIGN KEY (organization_id) REFERENCES organizations(id) ON DELETE CASCADE
);

CREATE TABLE posts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    board_id INTEGER NOT NULL,
    user_id INTEGER NOT NULL,
    title TEXT NOT NULL,
    content TEXT NOT NULL,
    problem_id INTEGER, -- 문제 관련 게시글인 경우
    contest_id INTEGER, -- 대회 관련 게시글인 경우
    is_pinned BOOLEAN DEFAULT 0, -- 공지 고정
    is_locked BOOLEAN DEFAULT 0, -- 댓글 잠금
    view_count INTEGER DEFAULT 0,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,

    FOREIGN KEY (board_id) REFERENCES boards(id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES users(id),
    FOREIGN KEY (contest_id) REFERENCES contests(id)
);

CREATE TABLE comments (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    post_id INTEGER NOT NULL,
    user_id INTEGER NOT NULL,
    parent_comment_id INTEGER, -- 대댓글 지원
    content TEXT NOT NULL,
    is_answer BOOLEAN DEFAULT 0, -- Q&A 채택 답변
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,

    FOREIGN KEY (post_id) REFERENCES posts(id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES users(id),
    FOREIGN KEY (parent_comment_id) REFERENCES comments(id) ON DELETE CASCADE
);

CREATE TABLE post_likes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    post_id INTEGER NOT NULL,
    user_id INTEGER NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,

    FOREIGN KEY (post_id) REFERENCES posts(id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES users(id),
    UNIQUE(post_id, user_id)
);

CREATE TABLE comment_likes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    comment_id INTEGER NOT NULL,
    user_id INTEGER NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,

    FOREIGN KEY (comment_id) REFERENCES comments(id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES users(id),
    UNIQUE(comment_id, user_id)
);

-- 게시판 인덱스
CREATE INDEX idx_posts_board_id ON posts(board_id);
CREATE INDEX idx_posts_user_id ON posts(user_id);
CREATE INDEX idx_posts_problem_id ON posts(problem_id);
CREATE INDEX idx_posts_contest_id ON posts(contest_id);
CREATE INDEX idx_posts_created_at ON posts(created_at DESC);
CREATE INDEX idx_posts_is_pinned ON posts(is_pinned);
CREATE INDEX idx_comments_post_id ON comments(post_id);
CREATE INDEX idx_comments_user_id ON comments(user_id);
CREATE INDEX idx_comments_parent_comment_id ON comments(parent_comment_id);
CREATE INDEX idx_post_likes_post_id ON post_likes(post_id);
CREATE INDEX idx_post_likes_user_id ON post_likes(user_id);
CREATE INDEX idx_comment_likes_comment_id ON comment_likes(comment_id);
CREATE INDEX idx_comment_likes_user_id ON comment_likes(user_id);

-- 기본 게시판 생성
INSERT INTO boards (name, board_type, description) VALUES
('공지사항', 'announcement', '시스템 공지사항'),
('자유게시판', 'free', '자유롭게 이야기를 나누는 공간입니다'),
('질문게시판', 'qna', '문제 관련 질문을 하는 곳입니다');

-- ============================================
-- 뷰
-- ============================================
CREATE VIEW submission_stats AS
SELECT
    problem_id,
    COUNT(*) as total_submissions,
    SUM(CASE WHEN status = 'ACCEPTED' THEN 1 ELSE 0 END) as accepted_submissions,
    CAST(SUM(CASE WHEN status = 'ACCEPTED' THEN 1 ELSE 0 END) AS FLOAT) / COUNT(*) * 100 as acceptance_rate,
    AVG(CASE WHEN status = 'ACCEPTED' THEN execution_time END) as avg_execution_time,
    AVG(CASE WHEN status = 'ACCEPTED' THEN memory_usage END) as avg_memory_usage
FROM submissions
GROUP BY problem_id;

-- ============================================
-- 트리거
-- ============================================
CREATE TRIGGER update_user_stats_after_submission
    AFTER UPDATE ON submissions
    WHEN NEW.status = 'ACCEPTED' AND OLD.status != 'ACCEPTED'
BEGIN
    INSERT INTO user_stats (user_id, total_solved, total_submissions, current_streak, max_streak, last_solved_date)
    VALUES (NEW.user_id, 1, 1, 1, 1, DATE('now'))
    ON CONFLICT(user_id) DO UPDATE SET
                                       total_solved = total_solved + 1,
                                       total_submissions = total_submissions + 1,
                                       current_streak = CASE
                                                            WHEN DATE(last_solved_date) = DATE('now', '-1 day') THEN current_streak + 1
                                                            ELSE 1
                                           END,
                                       max_streak = MAX(max_streak, current_streak + 1),
                                       last_solved_date = DATE('now');
END;

CREATE TRIGGER increment_total_submissions
    AFTER INSERT ON submissions
BEGIN
    INSERT INTO user_stats (user_id, total_submissions)
    VALUES (NEW.user_id, 1)
    ON CONFLICT(user_id) DO UPDATE SET
        total_submissions = total_submissions + 1;
END;
