use bollard::Docker;
use bollard::container::{
    CreateContainerOptions, Config, StartContainerOptions,
    WaitContainerOptions, RemoveContainerOptions
};
use bollard::models::{ContainerConfig, HostConfig, Mount, MountTypeEnum};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::fs;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct JudgeRequest {
    pub submission_id: i64,
    pub language: String,
    pub source_code: String,
    pub problem_id: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JudgeResult {
    pub status: String,
    pub score: i32,
    pub execution_time: Option<i32>,
    pub memory_usage: Option<i32>,
    pub compile_message: Option<String>,
    pub runtime_error_type: Option<String>,
    pub runtime_error_message: Option<String>,
    pub total_testcases: i32,
    pub passed_testcases: i32,
    pub testcase_results: Vec<TestcaseResultData>,
    pub compile_errors: Option<Vec<CompileErrorData>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TestcaseResultData {
    pub testcase_number: i32,
    pub status: String,
    pub execution_time: Option<i32>,
    pub memory_usage: Option<i32>,
    pub error_message: Option<String>,
    pub expected_output: Option<String>,
    pub actual_output: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CompileErrorData {
    pub line_number: Option<i32>,
    pub column_number: Option<i32>,
    pub error_type: Option<String>,
    pub error_message: String,
}

pub async fn judge_submission(request: JudgeRequest) -> anyhow::Result<JudgeResult> {
    let session_id = Uuid::new_v4().to_string();
    let temp_dir = format!("/tmp/judge_{}", session_id);

    // 임시 디렉토리 생성
    fs::create_dir_all(&temp_dir).await?;

    // 소스 코드 파일 생성
    let source_file = match request.language.as_str() {
        "cpp" => format!("{}/Main.cpp", temp_dir),
        "python" => format!("{}/Main.py", temp_dir),
        "java" => format!("{}/Main.java", temp_dir),
        _ => return Err(anyhow::anyhow!("Unsupported language")),
    };

    fs::write(&source_file, &request.source_code).await?;

    // Docker로 채점 실행
    let result = run_docker_judge_with_bollard(&request.language, &temp_dir, request.problem_id).await?;

    // 임시 파일 정리
    let _ = fs::remove_dir_all(&temp_dir).await;

    Ok(result)
}

async fn run_docker_judge_with_bollard(
    language: &str,
    temp_dir: &str,
    problem_id: u32
) -> anyhow::Result<JudgeResult> {
    let docker = Docker::connect_with_local_defaults()?;

    let docker_image = match language {
        "cpp" => "nekonic-judge-cpp:latest",
        "python" => "nekonic-judge-python:latest",
        "java" => "nekonic-judge-java:latest",
        _ => return Err(anyhow::anyhow!("Unsupported language")),
    };

    // 테스트케이스 경로 계산
    let folder_num = if problem_id == 0 { 0 } else { ((problem_id - 1) / 1000 + 1) * 1000 };
    let testcase_path = std::path::Path::new("./problems")
        .join(format!("{:06}", folder_num))
        .join(problem_id.to_string())
        .join("testcases");

    let testcase_path_str = testcase_path.to_string_lossy().to_string();

    // 컨테이너 생성 옵션
    let container_name = format!("judge_{}", Uuid::new_v4());

    let mut mounts = vec![
        Mount {
            target: Some("/workspace".to_string()),
            source: Some(temp_dir.to_string()),
            typ: Some(MountTypeEnum::BIND),
            read_only: Some(false),
            ..Default::default()
        }
    ];

    // 테스트케이스 디렉토리가 존재하는 경우에만 마운트
    if testcase_path.exists() {
        mounts.push(Mount {
            target: Some("/testcases".to_string()),
            source: Some(testcase_path_str),
            typ: Some(MountTypeEnum::BIND),
            read_only: Some(true),
            ..Default::default()
        });
    }

    let host_config = HostConfig {
        memory: Some(512 * 1024 * 1024), // 512MB
        cpu_quota: Some(100000), // 1 CPU core
        network_mode: Some("none".to_string()),
        mounts: Some(mounts),
        ..Default::default()
    };

    let config = Config {
        image: Some(docker_image),
        working_dir: Some("/workspace"),
        host_config: Some(host_config),
        ..Default::default()
    };

    let options = CreateContainerOptions {
        name: container_name.clone(),
        platform: None,
    };

    // 컨테이너 생성 및 실행
    let container = docker.create_container(Some(options), config).await?;

    docker.start_container(&container.id, None::<StartContainerOptions<String>>).await?;

    // 컨테이너 종료 대기 (타임아웃 설정)
    let wait_options = WaitContainerOptions {
        condition: "not-running",
    };

    let mut wait_stream = docker.wait_container(&container.id, Some(wait_options));

    // 30초 타임아웃
    let wait_result = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        wait_stream.next()
    ).await;

    // 컨테이너 로그 수집
    let logs = docker.logs(
        &container.id,
        Some(bollard::container::LogsOptions::<String> {
            stdout: true,
            stderr: true,
            ..Default::default()
        }),
    );

    let mut stdout_output = String::new();
    let mut stderr_output = String::new();

    use futures_util::stream::StreamExt;
    let mut log_stream = logs;
    while let Some(log_result) = log_stream.next().await {
        match log_result {
            Ok(log_output) => {
                match log_output {
                    bollard::container::LogOutput::StdOut { message } => {
                        stdout_output.push_str(&String::from_utf8_lossy(&message));
                    },
                    bollard::container::LogOutput::StdErr { message } => {
                        stderr_output.push_str(&String::from_utf8_lossy(&message));
                    },
                    _ => {}
                }
            },
            Err(_) => break,
        }
    }

    // 컨테이너 제거
    docker.remove_container(
        &container.id,
        Some(RemoveContainerOptions {
            force: true,
            ..Default::default()
        })
    ).await?;

    // 결과 분석
    parse_judge_result(&stdout_output, &stderr_output, wait_result.is_ok())
}

fn parse_judge_result(
    stdout: &str,
    stderr: &str,
    completed_normally: bool
) -> anyhow::Result<JudgeResult> {
    if !completed_normally {
        return Ok(JudgeResult {
            status: "TIME_LIMIT_EXCEEDED".to_string(),
            score: 0,
            execution_time: None,
            memory_usage: None,
            compile_message: None,
            runtime_error_type: Some("TIME_LIMIT_EXCEEDED".to_string()),
            runtime_error_message: Some("채점 시간이 초과되었습니다.".to_string()),
            total_testcases: 1,
            passed_testcases: 0,
            testcase_results: vec![],
            compile_errors: None,
        });
    }

    if stderr.contains("COMPILATION_ERROR") || stdout.contains("COMPILATION_ERROR") {
        return Ok(JudgeResult {
            status: "COMPILATION_ERROR".to_string(),
            score: 0,
            execution_time: None,
            memory_usage: None,
            compile_message: Some(stderr.to_string()),
            runtime_error_type: None,
            runtime_error_message: None,
            total_testcases: 0,
            passed_testcases: 0,
            testcase_results: vec![],
            compile_errors: Some(vec![CompileErrorData {
                line_number: None,
                column_number: None,
                error_type: Some("COMPILATION_ERROR".to_string()),
                error_message: stderr.to_string(),
            }]),
        });
    }

    if stdout.contains("ACCEPTED") {
        Ok(JudgeResult {
            status: "ACCEPTED".to_string(),
            score: 100,
            execution_time: Some(100), // 실제로는 judge.sh에서 측정된 값 파싱
            memory_usage: Some(1024),
            compile_message: None,
            runtime_error_type: None,
            runtime_error_message: None,
            total_testcases: 1,
            passed_testcases: 1,
            testcase_results: vec![TestcaseResultData {
                testcase_number: 1,
                status: "ACCEPTED".to_string(),
                execution_time: Some(100),
                memory_usage: Some(1024),
                error_message: None,
                expected_output: None,
                actual_output: None,
            }],
            compile_errors: None,
        })
    } else if stdout.contains("WRONG_ANSWER") {
        Ok(JudgeResult {
            status: "WRONG_ANSWER".to_string(),
            score: 0,
            execution_time: Some(100),
            memory_usage: Some(1024),
            compile_message: None,
            runtime_error_type: None,
            runtime_error_message: None,
            total_testcases: 1,
            passed_testcases: 0,
            testcase_results: vec![TestcaseResultData {
                testcase_number: 1,
                status: "WRONG_ANSWER".to_string(),
                execution_time: Some(100),
                memory_usage: Some(1024),
                error_message: Some("출력이 예상과 다릅니다.".to_string()),
                expected_output: None,
                actual_output: None,
            }],
            compile_errors: None,
        })
    } else {
        Ok(JudgeResult {
            status: "RUNTIME_ERROR".to_string(),
            score: 0,
            execution_time: None,
            memory_usage: None,
            compile_message: None,
            runtime_error_type: Some("RUNTIME_ERROR".to_string()),
            runtime_error_message: Some(stderr.to_string()),
            total_testcases: 1,
            passed_testcases: 0,
            testcase_results: vec![],
            compile_errors: None,
        })
    }
}