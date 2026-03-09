
1. Identify existing static analysis tools that support line limit checks across various programming languages and text-based artifacts.
2. Investigate the capabilities of general-purpose static analysis tools like SonarQube, Checkstyle, and PMD to determine if they support line limit checks.
3. Explore language-specific static analysis tools and their plugins or extensions to see if any offer line limit checking functionality.
4. Examine the documentation and feature sets of static analysis tools for popular programming languages (e.g., Java, Python, C#) to find line limit checking capabilities.
5. Research static analysis tools for text-based artifacts like Markdown files to see if they support line limit checks.
6. Investigate the possibility of using scripting languages (e.g., Python, Bash) to create custom static analysis tools that check for line limits in various file types.
7. Explore the use of regular expressions and text processing tools to create simple line limit checks for different file types.
8. Examine the integration capabilities of static analysis tools with continuous integration (CI) pipelines to ensure line limit checks can be automated and enforced during the development process.
# Non-Language-Specific Static Analysis Tools for Enforcing Line Count Limits Across Text-Based Files

> - Few static analysis tools natively enforce configurable line count limits across arbitrary text files.  
> - `cloc` counts lines of code in many languages but lacks direct line limit enforcement.  
> - `Semgrep` supports custom rules and could enforce line limits via regex, but requires rule development.  
> - `SonarQube` enforces line limits only in the context of licensing, not per-file limits.  
> - Custom scripting (e.g., Bash/Python) combined with `cloc` or regex is the most flexible approach.  

---

## Introduction

Maintaining concise and manageable file sizes is a critical aspect of software quality assurance (QA), especially in large projects with diverse file types such as Markdown documentation, source code (Python, Rust, JavaScript), and configuration files (YAML, TOML). Excessively long files hinder readability, maintainability, and collaboration. To enforce line count limits uniformly across various text-based artifacts, developers need static analysis tools that are both cross-language and configurable, with seamless integration into QA workflows including CI/CD pipelines.

This report evaluates existing non-language-specific static analysis tools capable of enforcing line count limits across diverse text files. It assesses their support for configurable thresholds, integration capabilities, reporting formats, and performance. The goal is to identify the most suitable off-the-shelf solutions or combinations thereof, and to outline custom solution approaches when necessary.

---

## Evaluation of Static Analysis Tools for Line Limit Enforcement

### 1. cloc (Count Lines of Code)

**Summary**:  
`cloc` is a widely used open-source tool that counts lines of code (LOC) across many programming languages and file types. It distinguishes between blank lines, comment lines, and actual code lines, providing detailed reports in various formats (console, CSV, XML, YAML). While `cloc` does not natively enforce line limits, it can count lines in any text file and supports exclusion patterns and language-specific filters.

**Strengths**:  
- Cross-language and cross-format support (including Markdown, YAML, Python, Rust, JavaScript).  
- Configurable via command-line options and config files.  
- Integrates well with CI/CD pipelines (e.g., GitHub Actions).  
- Outputs parseable formats suitable for automated QA workflows.  
- Actively maintained, open-source (GPLv2).  

**Limitations**:  
- Does not directly enforce line limits or generate warnings/errors when thresholds are exceeded.  
- Requires post-processing (e.g., scripting) to compare counts against thresholds and fail builds.  

**Use Case**:  
Best suited as a foundational component in a custom solution where line counts are extracted and compared against configurable limits via scripting.

---

### 2. Semgrep

**Summary**:  
Semgrep is an open-source static analysis tool supporting over 30 programming languages. It combines AST and regex-based pattern matching to enforce coding standards and security policies. While Semgrep does not natively enforce line count limits, its flexible rule system allows users to define custom rules that could detect and report files exceeding line thresholds.

**Strengths**:  
- Extensible via custom rules written in YAML or Python.  
- Supports complex pattern matching including regex on file content.  
- Integrates with CI/CD pipelines and GitHub Actions.  
- Outputs findings in JSON and JUnit XML formats.  
- Free tier available, open-source core.  

**Limitations**:  
- Requires custom rule development to enforce line limits, which may be complex.  
- Performance may degrade on very large files or complex rules.  
- Not specifically designed for line counting or file size limits.  

**Use Case**:  
Suitable for teams willing to invest in custom rule development to enforce line limits as part of a broader static analysis workflow.

---

### 3. SonarQube

**Summary**:  
SonarQube is a comprehensive static analysis platform supporting 25+ languages. It tracks lines of code (LOC) primarily for licensing compliance, notifying users when total LOC exceeds license thresholds. While SonarQube can monitor LOC at the project level, it does not natively enforce per-file line limits.

**Strengths**:  
- Centralized management and reporting.  
- Extensible via plugins, including custom rule development.  
- Integrates with CI/CD pipelines and IDEs.  
- Supports many languages and frameworks.  
- Commercial and open-source editions available.  

**Limitations**:  
- Line limit enforcement is tied to licensing, not per-file limits.  
- Requires plugin development or custom configuration to enforce file-level line limits.  
- More complex setup and maintenance compared to lightweight tools.  

**Use Case**:  
Best for large teams needing centralized code quality management who can invest in custom plugin development.

---

### 4. Checkstyle

**Summary**:  
Checkstyle is a Java-focused static analysis tool that enforces coding standards, including line length limits via its `LineLength` module. It can also check file length (number of lines) through its `FileLength` module, which can be configured via XML to enforce maximum line counts per file.

**Strengths**:  
- Supports configurable line length and file length limits.  
- Integrates with CI/CD pipelines and IDEs.  
- Outputs in Checkstyle XML format, compatible with many tools.  
- Open-source, actively maintained.  

**Limitations**:  
- Primarily Java-focused; limited support for other languages and file types.  
- Requires XML configuration, which may be less flexible than JSON/YAML.  
- Not designed for non-code text files like Markdown or YAML.  

**Use Case**:  
Ideal for Java projects requiring strict line limit enforcement but less suitable for multi-language or non-code text files.

---

### 5. PMD

**Summary**:  
PMD is a Java static analysis tool that includes rules for excessive class length (line count) and can be configured to enforce line limits. It supports multiple output formats and integrates with build tools and CI/CD pipelines.

**Strengths**:  
- Supports line count checks via `ExcessiveClassLength` rule.  
- Configurable through XML.  
- Integrates with CI/CD pipelines.  
- Open-source.  

**Limitations**:  
- Java-specific, not suitable for other languages or non-code files.  
- Less flexible configuration compared to JSON/YAML.  

**Use Case**:  
Useful for Java projects but not applicable to the broader multi-language and multi-file-type requirement.

---

### 6. markdownlint

**Summary**:  
`markdownlint` is a Node.js-based linter for Markdown files that checks for style issues, including long lines via rule MD013. It supports custom rules and configuration via style files, enabling enforcement of line length limits in Markdown.

**Strengths**:  
- Specifically designed for Markdown files.  
- Configurable line length limits.  
- Supports custom rules and integration into CI/CD pipelines.  
- Open-source.  

**Limitations**:  
- Limited to Markdown files only.  
- Requires Node.js environment.  
- Not suitable for source code or other file types.  

**Use Case**:  
Best for enforcing line limits in Markdown documentation as part of a QA workflow.

---

### 7. Custom Scripting Solutions

**Summary**:  
Custom scripts (Bash, Python) can leverage tools like `cloc`, `grep`, or `wc -l` to count lines in files and enforce thresholds. These scripts can be integrated into CI/CD pipelines as pre-commit hooks or standalone checks.

**Strengths**:  
- Complete flexibility to support any file type and configurable thresholds.  
- Can integrate with existing tools and pipelines.  
- No dependency on language-specific linters.  
- Open-source and fully customizable.  

**Limitations**:  
- Requires development and maintenance effort.  
- May lack advanced features like detailed reporting or IDE integration.  
- Performance depends on implementation.  

**Use Case**:  
Ideal when no single off-the-shelf tool meets all requirements or when maximum flexibility is needed.

---

## Comparative Table of Relevant Tools

| Tool Name       | Supported File Types                | Configuration Method          | Integration                   | Reporting Format           | License           | Notes                                                                                  |
|-----------------|-----------------------------------|------------------------------|-------------------------------|----------------------------|-------------------|----------------------------------------------------------------------------------------|
| cloc            | Any text file, 100+ languages     | CLI options, config files    | CLI, CI/CD pipelines          | Console, CSV, XML, YAML    | Open-source (GPL) | Counts lines but does not enforce limits; requires post-processing                      |
| Semgrep          | 30+ programming languages          | Custom rules (YAML/Python)   | CLI, CI/CD pipelines          | JSON, JUnit XML            | Open-source       | Requires custom rule development to enforce line limits                               |
| SonarQube        | 25+ programming languages          | Web UI, plugins              | CI/CD, IDE                   | Web dashboard, JSON, XML   | Open-source + Paid| Enforces LOC limits for licensing, not per-file limits; extensible via plugins         |
| Checkstyle       | Java files                        | XML configuration            | CLI, CI/CD, IDE              | Checkstyle XML             | Open-source       | Supports line length and file length limits; Java-specific                             |
| PMD              | Java files                        | XML configuration            | CLI, CI/CD                   | XML, HTML, others          | Open-source       | Supports excessive class length checks; Java-specific                                 |
| markdownlint     | Markdown files                   | Style files, CLI options     | CLI, CI/CD                   | Console, JSON              | Open-source       | Enforces line length limits in Markdown; not for other file types                      |
| Custom Scripts   | Any file type                    | Scripting                    | CLI, CI/CD                   | Console                   | Open-source       | Fully customizable; requires development effort                                       |

---

## Recommendations

### For Immediate Off-the-Shelf Solutions

- **Use `cloc` for line counting combined with custom scripting** to enforce thresholds in a CI/CD pipeline. This approach leverages `cloc`’s broad language support and integrates easily into QA workflows.  
- **Use `markdownlint` for Markdown-specific line limit enforcement** if Markdown files are a primary concern.  
- **Use `Checkstyle` or `PMD` for Java projects** where line limits are critical and language-specific tools are acceptable.  

### For Teams Willing to Invest in Customization

- **Develop custom Semgrep rules** to enforce line limits across multiple languages, leveraging Semgrep’s powerful pattern matching and CI/CD integration.  
- **Develop a SonarQube plugin** to enforce per-file line limits if centralized code quality management is a priority.  
- **Write custom Bash/Python scripts** that wrap `cloc`, `wc -l`, or regex-based checks to enforce thresholds and integrate into pre-commit hooks or CI/CD pipelines.  

### Example Custom Script Approach

```bash
#!/bin/bash

# Define line limits per file type
declare -A LINE_LIMITS=(
    ["md"]=100
    ["py"]=300
    ["rs"]=300
    ["js"]=500
)

# Iterate over files and enforce limits
for file in $(find . -type f); do
    extension="${file##*.}"
    limit="${LINE_LIMITS[$extension]}"

    if [[ -n "$limit" ]]; then
        line_count=$(wc -l < "$file")
        if (( line_count > limit )); then
            echo "ERROR: $file exceeds line limit ($line_count > $limit)"
            exit 1
        fi
    fi
done

echo "All files within line limits."
exit 0
```

This script can be integrated into a pre-commit hook or CI/CD pipeline to fail builds when files exceed specified line limits.

---

## Final Conclusion

No single non-language-specific static analysis tool currently meets all the criteria for enforcing configurable line count limits across arbitrary text-based files while integrating seamlessly into QA workflows. However, combining existing tools like `cloc` with custom scripting or leveraging flexible tools like `Semgrep` with custom rules provides a practical path forward. For Java-specific projects, `Checkstyle` and `PMD` offer mature solutions. For Markdown files, `markdownlint` is highly effective.

The most universally applicable and maintainable approach is to develop a lightweight custom script or wrapper that integrates line counting tools and enforces thresholds, tailored to the specific file types and limits required. This approach ensures flexibility, performance, and seamless integration into CI/CD pipelines, enabling teams to maintain concise and manageable file sizes effectively.

---

This comprehensive analysis synthesizes the current landscape of static analysis tools relevant to line limit enforcement, providing actionable recommendations tailored to diverse QA workflows and file types.
