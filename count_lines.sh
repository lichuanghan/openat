#!/bin/bash

# 统计代码行数的脚本
# 支持常见编程语言文件

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# 显示使用帮助
show_help() {
    echo -e "${BLUE}用法: $0 [目录路径]${NC}"
    echo ""
    echo "示例:"
    echo "  $0              # 统计当前目录"
    echo "  $0 /path/to/dir # 统计指定目录"
    echo ""
    echo "支持的编程语言:"
    echo "  .sh   - Shell脚本"
    echo "  .py   - Python"
    echo "  .rs   - Rust"
    echo "  .js   - JavaScript"
    echo "  .ts   - TypeScript"
    echo "  .java - Java"
    echo "  .go   - Go"
    echo "  .c/.h - C/C++"
    echo "  .json - JSON"
    echo "  .md   - Markdown"
}

# 统计函数
count_lines() {
    local dir="${1:-.}"

    echo -e "${GREEN}正在统计目录: $dir${NC}"
    echo "========================================"
    echo ""

    # 定义要统计的文件类型
    local exts="*.rs *.py *.sh *.js *.ts *.java *.go *.c *.cpp *.h *.hpp *.json *.md *.yaml *.yml *.xml *.html *.css *.scss *.lua *.php *.rb *.swift *.kt *.sql"

    local total_lines=0
    local total_files=0

    # 使用临时文件存储统计结果
    local tmpfile=$(mktemp)

    for ext in $exts; do
        find "$dir" -type f -name "$ext" 2>/dev/null | while read -r file; do
            lines=$(wc -l < "$file" 2>/dev/null || echo 0)
            extname=$(echo "$ext" | sed 's/\*//')
            echo "$extname $lines" >> "$tmpfile"
        done
    done

    # 按扩展名分组统计
    if [ -f "$tmpfile" ]; then
        echo -e "${YELLOW}文件类型统计:${NC}"
        echo "----------------------------------------"
        printf "%-10s %20s\n" "扩展名" "行数"
        printf "%-10s %20s\n" "--------" "--------------------"

        # 统计每种类型
        while read -r ext lines; do
            echo "$ext $lines"
        done < "$tmpfile" | awk '{
            ext=$1
            lines=$2
            sum[ext]+=lines
            count[ext]++
        }
        END {
            for (e in sum) {
                printf "  %-8s %15d 行 (%d 文件)\n", e, sum[e], count[e]
                total+=sum[e]
                files+=count[e]
            }
        }' | sort -t' ' -k3 -rn

        echo "----------------------------------------"
        total=$(awk '{sum+=$2} END {print sum}' "$tmpfile")
        total_files=$(awk '{count++} END {print count}' "$tmpfile")
        echo ""
        echo -e "${GREEN}总计: $total_files 个文件, $total 行代码${NC}"

        rm -f "$tmpfile"
    fi
}

# 主程序
main() {
    # 检查参数
    if [ "$1" = "-h" ] || [ "$1" = "--help" ]; then
        show_help
        exit 0
    fi

    # 设置目录路径
    target_dir="${1:-.}"

    # 检查目录是否存在
    if [ ! -d "$target_dir" ]; then
        echo -e "${RED}错误: 目录 '$target_dir' 不存在${NC}"
        exit 1
    fi

    # 获取绝对路径
    target_dir=$(cd "$target_dir" && pwd)

    # 开始统计
    count_lines "$target_dir"
}

# 执行主程序
main "$@"
