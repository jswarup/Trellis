#!/usr/bin/env python3
# tools/format.py — Formatting and compliance check for Segue
#
# Rules:
# 1. Indentation & Line Endings: 4 spaces, Unix (LF). No literal tabs.
# 2. Keywords & Alignment: Spaces up to next tab stop (col % 4 == 0) after fn, let, and use.
# 3. Brace Placement: AlwaysNextLine for struct, impl, fn, and control flow (if, match, for, while, loop),
#    EXCEPT closures (|...| {) and ternary assignments (= if () {} else {}).
# 4. Spacing in Brackets: Open paren '(' has trailing space if not empty; open '<' has trailing space in generics.
# 5. Local Variables: camelCase (let  myLocalVar = ...).
# 6. Separator Lines: //--- lines padded with blank line before and after.
# 7. Trailing Comments: Aligned to column 72.

import os
import sys
import re

def tab_align_after(keyword, col_end):
    # Tab stops are 4, 8, 12, 16...
    # Returns number of spaces to reach the next tab stop (at least 1, or 2 for fn/let)
    next_tab = ((col_end // 4) + 1) * 4
    diff = next_tab - col_end
    if diff == 0:
        diff = 4
    return diff

def chunk_line(line):
    chunks = []
    current = ""
    state = "CODE"
    i = 0
    while i < len(line):
        char = line[i]
        if state == "CODE":
            if char == '"':
                if current:
                    chunks.append(("CODE", current))
                    current = ""
                state = "STRING"
                current += char
            elif char == "'":
                if current:
                    chunks.append(("CODE", current))
                    current = ""
                state = "CHAR"
                current += char
            elif char == '/' and i + 1 < len(line) and line[i+1] == '/':
                if current:
                    chunks.append(("CODE", current))
                    current = ""
                chunks.append(("COMMENT", line[i:]))
                break
            else:
                current += char
        elif state == "STRING":
            current += char
            if char == '"' and (i == 0 or line[i-1] != '\\'):
                chunks.append(("STRING", current))
                current = ""
                state = "CODE"
        elif state == "CHAR":
            current += char
            if char == "'" and (i == 0 or line[i-1] != '\\'):
                chunks.append(("CHAR", current))
                current = ""
                state = "CODE"
        i += 1
    if current:
        chunks.append((state, current))
    return chunks

def align_inline_comment(line):
    chunks = chunk_line(line)
    if not chunks:
        return line
    comment_idx = -1
    for idx, (t, val) in enumerate(chunks):
        if t == "COMMENT":
            comment_idx = idx
            break
    if comment_idx == -1:
        return line
    before_chunks = chunks[:comment_idx]
    code_before = "".join(v for t, v in before_chunks)
    if not code_before.strip():
        return line
    comment_text = chunks[comment_idx][1]
    if re.search(r'^//[-=]{5,}', comment_text.strip()):
        return line
    code_before_stripped = code_before.rstrip()
    col = len(code_before_stripped)
    if col < 71:
        padding = " " * (71 - col)
    else:
        padding = " "
    return code_before_stripped + padding + comment_text

def format_line_code(line):
    chunks = chunk_line(line)
    formatted_line = ""
    for c_type, val in chunks:
        if c_type == "CODE":
            # Keyword fn: 2 spaces
            val = re.sub(r'\bfn\s+', 'fn  ', val)
            # Keyword let: 2 spaces (let  var)
            val = re.sub(r'\blet\s+', 'let  ', val)
            # Space after open parenthesis unless empty, already spaced, or end of line
            val = re.sub(r'\((?![ \)\n$])', '( ', val)
            # Space after open generic bracket (e.g. Arr< T>, Option< T>)
            val = re.sub(r'([a-zA-Z0-9_]+)<(?![ \>\n<=$])', r'\1< ', val)
        formatted_line += val
    return align_inline_comment(formatted_line)

def format_braces(line):
    # Splits braces to next line for struct, enum, impl, fn, if, else, match, for, while, loop
    # EXCEPT closures (|...| {) and ternary assignments (= if () {} else {})
    if not line.strip():
        return line
    chunks = chunk_line(line)
    has_brace = False
    brace_chunk_idx = -1
    brace_char_idx = -1
    for idx, (c_type, val) in enumerate(chunks):
        if c_type == "CODE" and "{" in val:
            remaining_code = "".join(c[1] for c in chunks[idx+1:] if c[0] == "CODE").strip()
            if not remaining_code:
                has_brace = True
                brace_chunk_idx = idx
                brace_char_idx = val.rfind("{")
                break
    if has_brace:
        before_chunks = chunks[:brace_chunk_idx]
        brace_chunk_val = chunks[brace_chunk_idx][1]
        before_brace_in_chunk = brace_chunk_val[:brace_char_idx]
        after_brace_in_chunk = brace_chunk_val[brace_char_idx+1:]
        after_chunks = chunks[brace_chunk_idx+1:]

        code_before = "".join(c[1] for c in before_chunks if c[0] == "CODE") + before_brace_in_chunk

        # Check exceptions:
        # 1. Closures: ends with | or ||
        if re.search(r'\|[^|]*\|\s*$', code_before.strip()):
            return line
        # 2. Ternary / if expression assignment: '= if' or 'let ... = if'
        if re.search(r'=\s*if\b', code_before):
            return line
        # 3. Trailing macro invocations like segue_test!(..., |...| { ... })
        if re.search(r'!(.*?,\s*\|.*?\|\s*)$', code_before.strip()):
            return line
        # 4. Macro definitions
        if "macro_rules!" in code_before:
            return line
        # 5. One-line ternary or return if: 'if ... { ... } else { ... }'
        if re.search(r'\bif\b.*\{.*\}\s*else', code_before):
            return line

        is_target_construct = bool(re.search(r'\b(struct|enum|impl|fn|if|else|match|for|while|loop)\b', code_before))
        # Also match function return type / parameter end: ') -> ... {' or ') {'
        is_fn_end = bool(re.search(r'\)\s*(->\s*[^{]+)?$', code_before.strip()))

        if (is_target_construct or is_fn_end) and code_before.strip():
            line_before = ""
            for t, v in before_chunks:
                line_before += v
            line_before += before_brace_in_chunk
            line_before = line_before.rstrip()

            indent = line[:len(line) - len(line.lstrip())]
            line_brace = indent + "{"

            line_after = after_brace_in_chunk
            for t, v in after_chunks:
                line_after += v
            line_after = line_after.strip()

            if line_after:
                return line_before + "\n" + line_brace + " " + line_after + "\n"
            else:
                return line_before + "\n" + line_brace + "\n"
    return line

def format_file_content(content):
    # 1. Normalize line endings to LF
    content = content.replace("\r\n", "\n").replace("\r", "\n")

    # 2. Remove tabs
    content = content.replace("\t", "    ")

    lines = content.splitlines()
    formatted_lines = []

    for line in lines:
        braced = format_braces(line)
        sub_lines = braced.splitlines()
        for sub_line in sub_lines:
            # Strip trailing whitespace on each formatted line
            formatted_lines.append(format_line_code(sub_line).rstrip())

    # 3. Pass 2: Ensure blank lines around //--- separator lines
    final_lines = []
    i = 0
    while i < len(formatted_lines):
        line = formatted_lines[i]
        is_separator = bool(re.match(r'^\s*//[-=]{5,}\s*$', line))
        if is_separator:
            if final_lines and final_lines[-1].strip():
                final_lines.append("")
            final_lines.append(line)
            if i + 1 < len(formatted_lines) and formatted_lines[i+1].strip():
                final_lines.append("")
        else:
            final_lines.append(line)
        i += 1

    new_content = "\n".join(final_lines)
    if content.endswith("\n") and not new_content.endswith("\n"):
        new_content += "\n"

    return new_content

def check_file(file_path):
    with open(file_path, "r", encoding="utf-8", errors="ignore") as f:
        content = f.read()

    issues = []
    # Check for CRLF
    if "\r" in content:
        issues.append("CRLF line endings detected")
    # Check for tabs
    if "\t" in content:
        issues.append("Literal tab character(s) detected")

    formatted = format_file_content(content)
    if content != formatted:
        issues.append("Formatting differs from standard rules (braces, keyword/bracket spacing, comments)")

    return issues

def main():
    check_mode = "--check" in sys.argv
    base_dir = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
    src_dir = os.path.join(base_dir, "src")
    tests_dir = os.path.join(base_dir, "tests")
    examples_dir = os.path.join(base_dir, "examples")

    dirs_to_process = [src_dir, tests_dir, examples_dir]
    total_files = 0
    non_compliant = 0

    for target_dir in dirs_to_process:
        if not os.path.exists(target_dir):
            continue
        for root, _, files in os.walk(target_dir):
            for file in files:
                if file.endswith(".rs"):
                    total_files += 1
                    file_path = os.path.join(root, file)
                    rel_path = os.path.relpath(file_path, base_dir)

                    if check_mode:
                        issues = check_file(file_path)
                        if issues:
                            non_compliant += 1
                            print(f"[FAIL] {rel_path}:")
                            for issue in issues:
                                print(f"       - {issue}")
                    else:
                        with open(file_path, "r", encoding="utf-8", errors="ignore") as f:
                            content = f.read()
                        new_content = format_file_content(content)
                        if content != new_content:
                            with open(file_path, "w", encoding="utf-8", newline="\n") as f:
                                f.write(new_content)
                            print(f"[FORMATTED] {rel_path}")

    if check_mode:
        print(f"\nChecked {total_files} Rust files: {total_files - non_compliant} compliant, {non_compliant} non-compliant.")
        sys.exit(1 if non_compliant > 0 else 0)
    else:
        print(f"\nProcessed {total_files} Rust files.")

if __name__ == "__main__":
    main()

