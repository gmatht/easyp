#!/bin/bash
#

# A git hook script to find and fix trailing whitespace
# in your commits. Bypass it with the --no-verify option
# to git-commit
#
# usage: make a soft link to this file, e.g., ln -s ~/config/pre-commit.git.sh ~/some_project/.git/hooks/pre-commit

# detect platform
platform="win"
uname_result=`uname`
if [ "$uname_result" = "Linux" ]; then
  platform="linux"
elif [ "$uname_result" = "Darwin" ]; then
  platform="mac"
fi

# change IFS to ignore filename's space in |for|
IFS="
"
# autoremove trailing whitespace - get unique files first
files_to_process=()
for line in `git diff --check --cached | sed '/^[+-]/d'` ; do
  # get file name
  if [ "$platform" = "mac" ]; then
    file="`echo $line | sed -E 's/:[0-9]+: .*//'`"
  else
    file="`echo $line | sed -r 's/:[0-9]+: .*//'`"
  fi
  
  # check if file has .toml, .lock, or .rs extension and not already in array
  if [[ "$file" =~ \.(toml|lock|rs)$ ]] && [[ ! " ${files_to_process[@]} " =~ " ${file} " ]]; then
    files_to_process+=("$file")
  fi
done

# process each file once
for file in "${files_to_process[@]}"; do
  echo -e "auto remove trailing whitespace in \033[31m$file\033[0m!"
  # since $file in working directory isn't always equal to $file in index, so we backup it
  mv -f "$file" "${file}.save"
  # discard changes in working directory
  git checkout -- "$file"
  # remove trailing whitespace and convert line endings to UNIX (LF) in one pass
  if [ "$platform" = "win" ]; then
    # in windows, `sed -i` adds ready-only attribute to $file(I don't kown why), so we use temp file instead
    sed -e 's/[[:space:]]*$//' -e 's/\r$//' "$file" > "${file}.bak"
    mv -f "${file}.bak" "$file"
  elif [ "$platform" == "mac" ]; then
    sed -i "" -e 's/[[:space:]]*$//' -e 's/\r$//' "$file"
  else
    sed -i -e 's/[[:space:]]*$//' -e 's/\r$//' "$file"
  fi  
  git add "$file"
  # restore the $file
  sed -e 's/[[:space:]]*$//' -e 's/\r$//' "${file}.save" > "$file"
  rm "${file}.save"
done

if [ "x`git status -s | grep '^[A|D|M]'`" = "x" ]; then
  # empty commit
  echo
  echo -e "\033[31mNO CHANGES ADDED, ABORT COMMIT!\033[0m"
  exit 1
fi

# Now we can commit
exit