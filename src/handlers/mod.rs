pub mod auth;
pub mod contests;
pub mod home;
pub mod problems;
pub mod rankings;
pub mod submissions;
pub mod admin;
pub mod organizations;

// Re-export for convenience
pub use auth::*;
pub use contests::{
    contests_page, contest_detail, create_contest_page, create_contest_action,
    register_contest, contest_standings, manage_contest, add_contest_problem,
    remove_contest_problem, submit_contest_problem, contest_problem_detail,
};
pub use home::*;
pub use problems::*;
pub use rankings::*;
pub use submissions::*;
// admin과 organizations는 명시적으로 함수만 re-export
pub use admin::{
    admin_dashboard, pending_organizations, review_organization,
    create_organization_admin, add_member_to_organization,
    pending_join_requests, review_join_request, promote_to_admin,
};
pub use organizations::{
    list_organizations, organization_detail, create_organization,
    request_join_organization, my_organizations, delete_organization,
    promote_to_group_admin, demote_to_member, remove_member, invite_member,
};
