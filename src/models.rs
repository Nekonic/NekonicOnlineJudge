use serde::{Deserialize, Serialize};

// --- Problem Models ---

#[derive(Debug, Serialize)]
pub struct ProblemListItem {
    pub id: u32,
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
    pub time_limit: String,
    pub memory_limit: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ProblemDetail {
    pub id: u32,
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
    pub time_limit: String,
    pub memory_limit: String,
    pub tags: Vec<String>,
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
    pub id: u32,
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
    pub sort_by: Option<String>,
    #[serde(default)]
    pub sort_order: Option<String>,
}

fn default_user_type() -> String {
    "individual".to_string()
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct RankingEntry {
    pub rank: i64,
    pub username: String,
    pub rating: i32,
    pub solved_count: i64,
    pub user_type: String,
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
    pub max_participants: Option<i32>,
}

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
