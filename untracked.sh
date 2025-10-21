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

# Find .rs files that exist but aren't tracked by git
run_find . -name "*.rs" -type f | run_sort > /tmp/all_rs_files.txt
{
    # Main repository
    git ls-files "*.rs" 2>/dev/null || true
    
    # All submodules
    for git_dir in $(run_find . -name ".git" -type d); do
        if [ "$git_dir" != ".git" ]; then
            submodule_dir=$(dirname "$git_dir")
            (cd "$submodule_dir" && git ls-files "*.rs" 2>/dev/null | sed "s|^|$submodule_dir/|") || true
        fi
    done
} | run_sort > /tmp/git_rs_files.txt

# Find untracked files
run_comm -23 /tmp/all_rs_files.txt /tmp/git_rs_files.txt > /tmp/untracked_rs_files.txt

# Show only the missing files
run_comm -23 /tmp/all_rs_files.txt /tmp/git_rs_files.txt | grep -v "\.backup$" | grep -v "\.broken$" | tee /tmp/filtered_untracked.txt

untracked_count=$(wc -l < /tmp/filtered_untracked.txt)

# Ask user if they want to add untracked files
if [ $untracked_count -gt 0 ]; then
    echo
    echo "Found $untracked_count untracked .rs files."
    echo "Do you want to add them to their respective git repositories? (y/N)"
    read -r response
    
    if [[ "$response" =~ ^[Yy]$ ]]; then
        echo "Adding untracked files to git..."
        
        # Process each untracked file
        while IFS= read -r file; do
            if [ -f "$file" ]; then
                # Determine which git repository this file belongs to
                git_repo="."
                
                # Debug: show what git directories we found
                echo "DEBUG: Found git directories:"
                run_find . -name ".git" -type d | while read -r git_dir; do
                    echo "  $git_dir"
                done
                
                # Check if it's in a submodule (has its own .git directory)
                # Skip the root .git directory and check submodules first
                for git_dir in $(run_find . -name ".git" -type d); do
                    submodule_dir=$(dirname "$git_dir")
                    echo "DEBUG: Checking if $file starts with $submodule_dir"
                    # Skip the root directory (.)
                    if [ "$submodule_dir" != "." ]; then
                        if [[ "$file" == "$submodule_dir"* ]]; then
                            git_repo="$submodule_dir"
                            echo "DEBUG: Found match! Using $git_repo"
                            break
                        fi
                    fi
                done
                
                # For workspace members without their own .git, add to main repo
                # but use the relative path from the workspace root
                
                echo "Adding $file to $git_repo"
                
                # Add the file to the appropriate repository
                if [ "$git_repo" = "." ]; then
                    git add "$file"
                else
                    (cd "$git_repo" && git add "${file#$git_repo/}")
                fi
            fi
        done < /tmp/untracked_rs_files.txt
        
        echo "Files added to git. You may want to commit them:"
        echo "  git commit -m 'Add untracked Rust source files'"
        
        # Show status of each repository
        echo
        echo "=== Git status ==="
        echo "Main repository:"
        git status --porcelain | grep -E "\.rs$" || echo "  No .rs changes"
        
        # Check submodules
        for git_dir in $(run_find . -name ".git" -type d); do
            if [ "$git_dir" != ".git" ]; then
                submodule_dir=$(dirname "$git_dir")
                echo "Submodule $submodule_dir:"
                (cd "$submodule_dir" && git status --porcelain | grep -E "\.rs$") || echo "  No .rs changes"
            fi
        done
    else
        echo "Skipping git add. Files remain untracked."
    fi
else
    echo "No untracked .rs files found."
fi

# Cleanup
#rm -f /tmp/all_rs_files.txt /tmp/git_rs_files.txt /tmp/untracked_rs_files.txt /tmp/filtered_untracked.txt