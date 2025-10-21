#!/bin/bash
# untracked.sh - Find .rs files needed by Cargo but missing from git

set -e

# Function to run find with proper quoting and exclusions
run_find() {
    if [ -f "C:/Program Files/Git/usr/bin/find.exe" ]; then
        "C:/Program Files/Git/usr/bin/find.exe" "$@" -not -path "*/target/*" -not -path "*/.git/*" -not -path "*/node_modules/*" -not -path "*/.cargo/*"
    else
        find "$@" -not -path "*/target/*" -not -path "*/.git/*" -not -path "*/node_modules/*" -not -path "*/.cargo/*"
    fi
}

# Function to run sort with proper locale settings
run_sort() {
    if [ -f "C:/Program Files/Git/usr/bin/sort.exe" ]; then
        "C:/Program Files/Git/usr/bin/sort.exe" "$@"
    else
        sort "$@"
    fi
}

# Function to run comm with proper locale settings
run_comm() {
    if [ -f "C:/Program Files/Git/usr/bin/comm.exe" ]; then
        "C:/Program Files/Git/usr/bin/comm.exe" "$@"
    else
        comm "$@"
    fi
}

#echo "=== Summary ==="
# Create temporary files for counting
# run_find already excludes target/, .git/, and other build artifacts
run_find . -name "*.rs" -type f | run_sort > /tmp/all_rs_files.txt

# Get tracked files from main repo and all submodules
#echo "Collecting tracked .rs files from main repo and submodules..."
{
    # Main repository
    git ls-files "*.rs" 2>/dev/null || true
    
    # All submodules (directories containing .git)
    for git_dir in $(run_find . -name ".git" -type d); do
        if [ "$git_dir" != ".git" ]; then
            # Get the parent directory of .git (the submodule root)
            submodule_dir=$(dirname "$git_dir")
            echo "Checking submodule: $submodule_dir"
            (cd "$submodule_dir" && git ls-files "*.rs" 2>/dev/null | sed "s|^|$submodule_dir/|") || true
        fi
    done
} | run_sort > /tmp/git_rs_files.txt
#echo "Using comm: $(which comm)"
#echo "Using sort: $(which sort)"
#echo "First 5 lines of all_rs_files.txt:"
#head -5 /tmp/all_rs_files.txt
#echo "First 5 lines of git_rs_files.txt:"
#head -5 /tmp/git_rs_files.txt
run_comm -23 /tmp/all_rs_files.txt /tmp/git_rs_files.txt | tee /tmp/untracked_rs_files.txt
untracked_count=$(wc -l < /tmp/untracked_rs_files.txt)
echo "Untracked .rs files: $untracked_count" 1>&2

extension_count=$(run_find easyp-crate/extensions -name "*.rs" -type f 2>/dev/null | wc -l)
echo "Extension files: $extension_count"  1>&2

generated_count=$(run_find target -name "*.rs" -type f 2>/dev/null | wc -l)
echo "Generated files: $generated_count"  1>&2

