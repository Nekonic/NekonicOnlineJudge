#!/bin/bash
set -e

TIMEOUT=5

echo "Compiling..."

# Main 클래스명으로 통일 (백준 스타일)
if ! javac Main.java 2>compile_error.txt; then
    echo "COMPILATION_ERROR"
    cat compile_error.txt >&2
    exit 1
fi

echo "Compilation successful"

if [ ! -d "/testcases" ]; then
    echo "No testcases directory found"
    echo "ACCEPTED"
    exit 0
fi

total_cases=0
passed_cases=0

for input_file in /testcases/*.in; do
    if [ -f "$input_file" ]; then
        total_cases=$((total_cases + 1))
        output_file="${input_file%.in}.out"
        if timeout ${TIMEOUT}s java Main < "$input_file" > result.txt; then
            if [ -f "$output_file" ]; then
                if diff -w result.txt "$output_file" > /dev/null; then
                    passed_cases=$((passed_cases + 1))
                else
                    echo "WRONG_ANSWER"
                    exit 1
                fi
            else
                passed_cases=$((passed_cases + 1))
            fi
        else
            echo "RUNTIME_ERROR"
            exit 1
        fi
    fi
done

if [ $total_cases -eq 0 ]; then
    echo "ACCEPTED"
else
    echo "ACCEPTED"
fi

exit 0
