# /review

Review the contents of a specified directory and provide a comprehensive analysis of the code structure, patterns, and potential improvements.

## Usage

Specify a directory path to review. If no path is provided, reviews the current directory.

Examples:
- `/review src/` - Review the src directory
- `/review src/commands/` - Review the commands module
- `/review` - Review the current directory

## Steps to execute:

1. List all files in the specified directory recursively
2. Analyze the code structure and organization
3. Identify patterns and architectural decisions
4. Check for code quality issues:
   - Unused code or dependencies
   - Inconsistent naming conventions
   - Missing error handling
   - Potential performance issues
   - File size and complexity (consider splitting large files)
   - Module organization and separation of concerns
5. Suggest improvements based on the project's coding standards (CLAUDE.md)
6. Highlight any security concerns or best practice violations
7. Provide a summary with actionable recommendations

The review follows the principles outlined in CLAUDE.md:
- Memory safety over micro-optimization
- Code clarity and maintainability
- Clean code maintenance (removing unused elements)
- Proper error handling patterns