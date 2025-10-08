// 대회 순위 업데이트 유틸리티
use sqlx::SqlitePool;
use chrono::{DateTime, Utc, NaiveDateTime};

/// 날짜 파싱 헬퍼
fn parse_db_datetime(datetime_str: &str) -> Option<DateTime<Utc>> {
    // RFC3339 형식
    if let Ok(dt) = DateTime::parse_from_rfc3339(datetime_str) {
        return Some(dt.with_timezone(&Utc));
    }

    // SQLite datetime 형식들
    if let Ok(dt) = NaiveDateTime::parse_from_str(datetime_str, "%Y-%m-%dT%H:%M:%S") {
        return Some(dt.and_utc());
    }

    if let Ok(dt) = NaiveDateTime::parse_from_str(datetime_str, "%Y-%m-%d %H:%M:%S") {
        return Some(dt.and_utc());
    }

    if let Ok(dt) = NaiveDateTime::parse_from_str(datetime_str, "%Y-%m-%dT%H:%M") {
        return Some(dt.and_utc());
    }

    None
}

/// ICPC 스타일 순위 업데이트
/// - solved: 맞춘 문제 수
/// - penalty: 각 문제를 맞출 때까지 걸린 시간(분) + 틀린 횟수 * 20분
pub async fn update_standings(
    pool: &SqlitePool,
    contest_id: i64,
    user_id: i64,
) -> Result<(), sqlx::Error> {
    // 대회 시작 시간 가져오기
    let (start_time,): (String,) = sqlx::query_as(
        "SELECT start_time FROM contests WHERE id = ?"
    )
    .bind(contest_id)
    .fetch_one(pool)
    .await?;

    let contest_start = parse_db_datetime(&start_time).unwrap_or_else(|| Utc::now());

    // 대회에 포함된 문제 목록 가져오기
    let contest_problems: Vec<(i64,)> = sqlx::query_as(
        "SELECT problem_id FROM contest_problems WHERE contest_id = ?"
    )
    .bind(contest_id)
    .fetch_all(pool)
    .await?;

    let mut total_solved = 0;
    let mut total_penalty = 0;

    // 각 문제별로 점수와 패널티 계산
    for (problem_id,) in contest_problems {
        // 해당 문제의 모든 제출 조회 (시간 순)
        let submissions: Vec<(String, String)> = sqlx::query_as(
            r#"
            SELECT status, created_at
            FROM submissions
            WHERE contest_id = ? AND user_id = ? AND problem_id = ?
            ORDER BY created_at ASC
            "#
        )
        .bind(contest_id)
        .bind(user_id)
        .bind(problem_id)
        .fetch_all(pool)
        .await?;

        let mut problem_solved = false;
        let mut wrong_attempts = 0;
        let mut solve_time_minutes = 0;

        for (status, created_at) in submissions {
            if status == "ACCEPTED" {
                problem_solved = true;

                // 제출 시간 계산 (대회 시작부터 몇 분 후인지)
                if let Some(submit_time) = parse_db_datetime(&created_at) {
                    let duration = submit_time.signed_duration_since(contest_start);
                    solve_time_minutes = duration.num_minutes().max(0) as i32;
                }
                break;
            } else if status != "PENDING" && status != "JUDGING" {
                // 틀린 제출 카운트 (PENDING이나 JUDGING은 제외)
                wrong_attempts += 1;
            }
        }

        if problem_solved {
            total_solved += 1;
            // 패널티 = 문제를 푼 시간(분) + 틀린 횟수 * 20분
            total_penalty += solve_time_minutes + (wrong_attempts * 20);
        }
    }

    // contest_participants 테이블 업데이트
    sqlx::query(
        r#"
        UPDATE contest_participants
        SET total_score = ?, penalty_time = ?
        WHERE contest_id = ? AND user_id = ?
        "#
    )
    .bind(total_solved)
    .bind(total_penalty)
    .bind(contest_id)
    .bind(user_id)
    .execute(pool)
    .await?;

    Ok(())
}

/// 특정 문제에 대한 사용자의 제출 상태 조회
pub async fn get_problem_status(
    pool: &SqlitePool,
    contest_id: i64,
    user_id: i64,
    problem_id: i64,
) -> Result<ProblemStatus, sqlx::Error> {
    let submissions: Vec<(String, String)> = sqlx::query_as(
        r#"
        SELECT status, created_at
        FROM submissions
        WHERE contest_id = ? AND user_id = ? AND problem_id = ?
        ORDER BY created_at ASC
        "#
    )
    .bind(contest_id)
    .bind(user_id)
    .bind(problem_id)
    .fetch_all(pool)
    .await?;

    if submissions.is_empty() {
        return Ok(ProblemStatus {
            solved: false,
            attempts: 0,
            time_minutes: 0,
        });
    }

    let mut solved = false;
    let mut attempts = 0;
    let mut time_minutes = 0;

    for (status, _created_at) in submissions {
        if status == "ACCEPTED" {
            solved = true;
            break;
        } else if status != "PENDING" && status != "JUDGING" {
            attempts += 1;
        }
    }

    Ok(ProblemStatus {
        solved,
        attempts,
        time_minutes,
    })
}

#[derive(Debug)]
pub struct ProblemStatus {
    pub solved: bool,
    pub attempts: i32,
    pub time_minutes: i32,
}
