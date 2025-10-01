#!/bin/bash

set -e

TIMEOUT=5
MEMORY_LIMIT=512000  # KB

# 컴파일
echo "Compiling..."
if ! g++ -o solution solution.cpp -std=c++17 -O2 -Wall 2>compile_error.txt; then
    echo "COMPILATION_ERROR"
    cat compile_error.txt >&2
    exit 1
fi

echo "Compilation successful"

# 테스트케이스 디렉토리 확인
if [ ! -d "/testcases" ]; then
    echo "No testcases directory found"
    echo "ACCEPTED"  # 테스트케이스가 없으면 컴파일 성공으로 간주
    exit 0
fi

# 테스트케이스 실행
total_cases=0
passed_cases=0

for input_file in /testcases/*.in; do
    if [ -f "$input_file" ]; then
        total_cases=$((total_cases + 1))
        output_file="${input_file%.in}.out"

        echo "Running test case: $(basename $input_file)"

        # 시간 및 메모리 제한으로 실행
        if timeout ${TIMEOUT}s /usr/bin/time -v ./solution < "$input_file" > result.txt 2>time_output.txt; then
            # 출력 비교
            if [ -f "$output_file" ]; then
                if diff -w result.txt "$output_file" > /dev/null; then
                    echo "Test case $(basename $input_file): PASSED"
                    passed_cases=$((passed_cases + 1))
                else
                    echo "WRONG_ANSWER"
                    echo "Test case: $(basename $input_file)"
                    echo "Expected:"
                    cat "$output_file"
                    echo "Got:"
                    cat result.txt
                    exit 1
                fi
            else
                echo "Warning: Expected output file not found: $output_file"
                passed_cases=$((passed_cases + 1))  # 예상 출력이 없으면 통과로 간주
            fi
        else
            exit_code=$?
            if [ $exit_code -eq 124 ]; then
                echo "TIME_LIMIT_EXCEEDED"
            else
                echo "RUNTIME_ERROR"
                cat time_output.txt >&2
            fi
            exit 1
        fi
    fi
done

if [ $total_cases -eq 0 ]; then
    echo "No test cases found"
    echo "ACCEPTED"
else
    echo "All test cases passed: $passed_cases/$total_cases"
    echo "ACCEPTED"
fi

exit 0
