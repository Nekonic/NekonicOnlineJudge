use serde::{Deserialize, Serialize};

// Helper function for deserializing empty string as None
fn deserialize_optional_i32<'de, D>(deserializer: D) -> Result<Option<i32>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: Option<String> = Option::deserialize(deserializer)?;
    match s {
        Some(s) if s.trim().is_empty() => Ok(None),
        Some(s) => s.parse::<i32>().map(Some).map_err(serde::de::Error::custom),
        None => Ok(None),
    }
}

// --- Problem Models ---

#[derive(Debug, Serialize)]
pub struct ProblemListItem {
    pub id: i64,
    pub title: String,
    pub accuracy: f64,
}

#[derive(Debug, sqlx::FromRow)]
pub struct ProblemStats {
    pub problem_id: i64,
    pub total_submissions: i64,
    pub accepted_submissions: i64,
    pub acceptance_rate: Option<f64>,
    pub avg_execution_time: Option<f64>,
    pub avg_memory_usage: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct ProblemMeta {
    pub title: String,
    pub time_limit: u32,      // ms 단위
    pub memory_limit: u32,    // MB 단위
    pub tags: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ProblemDetail {
    pub id: i64,
    pub meta: ProblemMeta,
    pub content: String,
    pub example_inputs: Vec<String>,
    pub example_outputs: Vec<String>,
    pub total_submits: i64,
    pub correct_submits: i64,
    pub accuracy: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct FrontMatter {
    pub title: String,
    #[serde(deserialize_with = "deserialize_time_limit")]
    pub time_limit: u32,      // ms 단위
    #[serde(deserialize_with = "deserialize_memory_limit")]
    pub memory_limit: u32,    // MB 단위
    pub tags: Vec<String>,
}

// 시간 제한 파싱 (문자열 또는 숫자 모두 허용)
fn deserialize_time_limit<'de, D>(deserializer: D) -> Result<u32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;

    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringOrInt {
        String(String),
        Int(u32),
    }

    match StringOrInt::deserialize(deserializer)? {
        StringOrInt::Int(i) => Ok(i),
        StringOrInt::String(s) => {
            // "1000ms", "1000 ms", "1000", "1s" 등 파싱
            let s = s.trim().to_lowercase();
            let s = s.replace("ms", "").replace(" ", "");

            if s.ends_with('s') && !s.ends_with("ms") {
                // 초 단위 -> 밀리초 변환
                let num = s.trim_end_matches('s').parse::<u32>()
                    .map_err(|e| Error::custom(format!("Invalid time limit: {}", e)))?;
                Ok(num * 1000)
            } else {
                s.parse::<u32>()
                    .map_err(|e| Error::custom(format!("Invalid time limit: {}", e)))
            }
        }
    }
}

// 메모리 제한 파싱 (문자열 또는 숫자 모두 허용)
fn deserialize_memory_limit<'de, D>(deserializer: D) -> Result<u32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;

    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringOrInt {
        String(String),
        Int(u32),
    }

    match StringOrInt::deserialize(deserializer)? {
        StringOrInt::Int(i) => Ok(i),
        StringOrInt::String(s) => {
            // "512MB", "512 MB", "512", "256MB" 등 파싱
            let s = s.trim().to_uppercase();
            let s = s.replace("MB", "").replace(" ", "");

            s.parse::<u32>()
                .map_err(|e| Error::custom(format!("Invalid memory limit: {}", e)))
        }
    }
}

// --- Submission Models ---

#[derive(Debug, Deserialize)]
pub struct SubmitForm {
    pub language: String,
    pub source_code: String,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct SubmissionRow {
    pub id: i64,
    pub username: String,
    pub language: String,
    pub status: String,
    pub score: Option<i32>,
    pub execution_time: Option<i32>,
    pub memory_usage: Option<i32>,
    pub submitted_at: String,
}

#[derive(Debug, Serialize)]
pub struct ProblemStatusData {
    pub id: i64,
    pub title: String,
    pub submissions: Vec<SubmissionRow>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct SubmissionDetailRow {
    pub id: i64,
    pub problem_id: i64,
    pub username: String,
    pub language: String,
    pub status: String,
    pub score: Option<i32>,
    pub execution_time: Option<i32>,
    pub memory_usage: Option<i32>,
    pub compile_message: Option<String>,
    pub runtime_error_type: Option<String>,
    pub runtime_error_message: Option<String>,
    pub total_testcases: i32,
    pub passed_testcases: i32,
    pub submitted_at: String,
    pub judged_at: Option<String>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct TestcaseResultRow {
    pub testcase_number: i32,
    pub status: String,
    pub execution_time: Option<i32>,
    pub memory_usage: Option<i32>,
    pub error_message: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SubmissionDetailData {
    pub submission: SubmissionDetailRow,
    pub testcase_results: Vec<TestcaseResultRow>,
}

// --- Auth Models ---

#[derive(Deserialize)]
pub struct LoginForm {
    pub username: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct RegisterForm {
    pub username: String,
    pub password: String,
}

// --- Rankings Models ---

#[derive(Debug, Deserialize)]
pub struct RankingsQuery {
    #[serde(default = "default_user_type")]
    pub user_type: String,
    #[serde(default)]
    pub organization_id: Option<i64>,
    #[serde(default)]
    pub view_type: Option<String>,
    #[serde(default)]
    pub sort_by: Option<String>,
    #[serde(default)]
    pub sort_order: Option<String>,
}

fn default_user_type() -> String {
    "all".to_string()
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct RankingEntry {
    pub rank: i64,
    pub username: String,
    pub rating: i32,
    pub solved_count: i64,
    pub user_type: String,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct OrganizationRankingInfo {
    pub id: i64,
    pub name: String,
    pub r#type: String,
    pub member_count: i64,
    #[sqlx(default)]
    pub avg_rating: f64,
    #[sqlx(default)]
    pub total_rating: f64,
    #[sqlx(default)]
    pub total_solved: i64,
    #[sqlx(default)]
    pub rank: i64,
}

// --- Organization/Group Models ---

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Organization {
    pub id: i64,
    pub name: String,
    pub r#type: String,
    pub description: Option<String>,
    pub status: String,
    pub created_by: Option<i64>,
    pub approved_by: Option<i64>,
    pub approved_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateOrganizationForm {
    pub name: String,
    pub r#type: String,
    pub description: Option<String>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct OrganizationMember {
    pub id: i64,
    pub user_id: i64,
    pub username: String,
    pub role: String,
    pub status: String,
    pub joined_at: String,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct OrganizationJoinRequest {
    pub id: i64,
    pub organization_id: i64,
    pub user_id: i64,
    pub username: String,
    pub status: String,
    pub message: Option<String>,
    pub requested_at: String,
    pub reviewed_by: Option<i64>,
    pub reviewed_at: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct JoinOrganizationForm {
    pub message: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ReviewRequestForm {
    pub action: String, // "approve" or "reject"
}

#[derive(Debug, Deserialize)]
pub struct AddMemberForm {
    pub username: String,
    pub role: Option<String>,
}

// --- Contest Admin Models ---

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Contest {
    pub id: i64,
    pub title: String,
    pub description: Option<String>,
    pub start_time: String,
    pub end_time: String,
    pub contest_type: String,
    pub is_public: bool,
    pub max_participants: Option<i32>,
    pub status: String,
    pub requires_approval: bool,
    pub created_by: i64,
    pub approved_by: Option<i64>,
    pub approved_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateContestForm {
    pub title: String,
    pub description: Option<String>,
    pub start_time: String,
    pub end_time: String,
    pub contest_type: String,
    pub is_public: bool,
    #[serde(deserialize_with = "deserialize_optional_i32")]
    pub max_participants: Option<i32>,
}

// --- Contest Extended Models ---

#[derive(Debug, Serialize)]
pub struct ContestWithStats {
    pub id: i64,
    pub title: String,
    pub description: Option<String>,
    pub start_time: String,
    pub end_time: String,
    pub contest_type: String,
    pub is_public: bool,
    pub status: String,
    pub participant_count: i64,
    pub max_participants: Option<i32>,
    pub created_by: i64,
    pub creator_username: String,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ContestDetail {
    pub id: i64,
    pub title: String,
    pub description: Option<String>,
    pub start_time: String,
    pub end_time: String,
    pub contest_type: String,
    pub is_public: bool,
    pub max_participants: Option<i32>,
    pub status: String,
    pub requires_approval: bool,
    pub created_by: i64,
    pub creator_username: String,
    pub created_at: String,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ContestProblem {
    pub id: i64,
    pub problem_id: i64,
    pub problem_title: String,
    pub points: i32,
    pub problem_order: i32,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ContestParticipant {
    pub id: i64,
    pub contest_id: i64,
    pub user_id: i64,
    pub username: String,
    pub total_score: i32,
    pub penalty_time: i32,
    pub joined_at: String,
}

// ICPC 스타일 순위표 엔트리
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct StandingsEntry {
    pub rank: i64,
    pub user_id: i64,
    pub username: String,
    pub solved: i32,
    pub penalty: i32,
    pub total_score: i32,
}

// 문제별 제출 상태
#[derive(Debug, Serialize)]
pub struct ProblemSubmissionStatus {
    pub problem_id: i64,
    pub solved: bool,
    pub attempts: i32,
    pub time_minutes: i32,
}

// 상세 순위표 엔트리 (문제별 상태 포함)
#[derive(Debug, Serialize)]
pub struct DetailedStandingsEntry {
    pub rank: i64,
    pub user_id: i64,
    pub username: String,
    pub solved: i32,
    pub penalty: i32,
    pub total_score: i32,
    pub problem_statuses: Vec<ProblemSubmissionStatus>,
}

#[derive(Debug, Serialize)]
pub struct ContestStandings {
    pub contest: ContestDetail,
    pub problems: Vec<ContestProblem>,
    pub standings: Vec<DetailedStandingsEntry>,
}

#[derive(Debug, Deserialize)]
pub struct AddContestProblemForm {
    pub problem_id: i64,
    pub points: i32,
    pub problem_order: i32,
}

#[derive(Debug, Deserialize)]
pub struct UpdateContestForm {
    pub title: Option<String>,
    pub description: Option<String>,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub is_public: Option<bool>,
    pub max_participants: Option<i32>,
}

// --- Contest Admin Models ---

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct AdminAction {
    pub id: i64,
    pub admin_id: i64,
    pub admin_username: String,
    pub action_type: String,
    pub target_type: String,
    pub target_id: i64,
    pub details: Option<String>,
    pub created_at: String,
}
