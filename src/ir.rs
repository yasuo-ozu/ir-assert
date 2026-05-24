use crate::predicate::combinators::Property;

/// Represents a parsed basic block with instruction counts.
#[derive(Debug, Clone)]
pub struct BasicBlockIr {
    pub calls: Vec<String>,
    pub instructions: usize,
    pub allocas: usize,
    pub branches: usize,
    pub phi_nodes: usize,
}

/// Lightweight parsed representation of a function's IR.
#[derive(Debug, Clone)]
pub struct FunctionIr {
    pub name: String,
    pub blocks: Vec<BasicBlockIr>,
    pub raw: String,
}

impl FunctionIr {
    pub(crate) fn compute_property(&self, prop: Property) -> usize {
        match prop {
            Property::BasicBlocksLen => self.blocks.len(),
            Property::CallsLen => self.blocks.iter().map(|b| b.calls.len()).sum(),
            Property::InstructionsLen => self.blocks.iter().map(|b| b.instructions).sum(),
            Property::AllocasLen => self.blocks.iter().map(|b| b.allocas).sum(),
            Property::BranchesLen => self.blocks.iter().map(|b| b.branches).sum(),
            Property::PhiNodesLen => self.blocks.iter().map(|b| b.phi_nodes).sum(),
        }
    }

    pub(crate) fn compute_block_property(&self, block_idx: usize, prop: Property) -> usize {
        let block = &self.blocks[block_idx];
        match prop {
            Property::BasicBlocksLen => {
                unreachable!("BasicBlocksLen is not a block-level property")
            }
            Property::CallsLen => block.calls.len(),
            Property::InstructionsLen => block.instructions,
            Property::AllocasLen => block.allocas,
            Property::BranchesLen => block.branches,
            Property::PhiNodesLen => block.phi_nodes,
        }
    }
}

/// Parse a function body text into a FunctionIr.
/// The input is the text between `define ... {` and the closing `}`.
pub(crate) fn parse_function_body(name: &str, body: &str, raw: &str) -> FunctionIr {
    let mut blocks: Vec<BasicBlockIr> = Vec::new();
    let mut current_block: Option<BasicBlockIr> = None;

    for line in body.lines() {
        let trimmed = line.trim();

        // Skip empty lines and comments
        if trimmed.is_empty() || trimmed.starts_with(';') {
            continue;
        }

        // Detect basic block labels (non-indented lines containing ':' before any comment)
        // Labels can be: `name:`, `"quoted_name":`, `name: ; preds = ...`
        if !line.starts_with(' ') && !line.starts_with('\t') {
            // Strip comment portion
            let before_comment = if let Some(semi_idx) = trimmed.find(';') {
                trimmed[..semi_idx].trim()
            } else {
                trimmed
            };
            if before_comment.ends_with(':') && !before_comment.contains('=') {
                if let Some(block) = current_block.take() {
                    blocks.push(block);
                }
                current_block = Some(BasicBlockIr {
                    calls: Vec::new(),
                    instructions: 0,
                    allocas: 0,
                    branches: 0,
                    phi_nodes: 0,
                });
                continue;
            }
        }

        // If we haven't seen a label yet, start the first block
        if current_block.is_none() {
            current_block = Some(BasicBlockIr {
                calls: Vec::new(),
                instructions: 0,
                allocas: 0,
                branches: 0,
                phi_nodes: 0,
            });
        }

        let block = current_block.as_mut().unwrap();

        // Count instruction (indented lines that aren't labels or comments)
        if (line.starts_with(' ') || line.starts_with('\t')) && !trimmed.is_empty() {
            block.instructions += 1;

            // Detect instruction types
            if let Some(callee) = extract_call_target(trimmed) {
                block.calls.push(callee);
            }
            if is_alloca_instruction(trimmed) {
                block.allocas += 1;
            }
            if is_branch_instruction(trimmed) {
                block.branches += 1;
            }
            if is_phi_instruction(trimmed) {
                block.phi_nodes += 1;
            }
        }
    }

    if let Some(block) = current_block {
        blocks.push(block);
    }

    FunctionIr {
        name: name.to_string(),
        blocks,
        raw: raw.to_string(),
    }
}

/// Extract the call target name from a call/invoke instruction, if it is a
/// real (non-asm, non-intrinsic) call. Returns `None` for asm calls, llvm
/// intrinsics, indirect calls without a named target, or non-call lines.
fn extract_call_target(line: &str) -> Option<String> {
    let call_idx = find_call_keyword(line)?;
    let after_call = &line[call_idx..];

    // Exclude inline asm
    if after_call.contains("asm ") || after_call.contains("asm\"") {
        return None;
    }

    // Exclude llvm intrinsics
    if after_call.contains("@llvm.") {
        return None;
    }

    // Find the `@` symbol that starts the callee name
    let at_pos = after_call.find('@')?;
    let after_at = &after_call[at_pos + 1..];

    let name = if let Some(stripped) = after_at.strip_prefix('"') {
        // Quoted name: @"some name"
        let end = stripped.find('"')?;
        &stripped[..end]
    } else {
        // Unquoted name: @function_name(...)
        let end = after_at
            .find(['(', ' ', ',', ')'])
            .unwrap_or(after_at.len());
        &after_at[..end]
    };

    Some(name.to_string())
}

/// Find the position of the `call` keyword in an instruction line.
fn find_call_keyword(line: &str) -> Option<usize> {
    // Look for standalone "call " or "invoke "
    for (i, _) in line.match_indices("call ") {
        // Ensure it's not part of another word (like "nocallback")
        if i == 0 || !line.as_bytes()[i - 1].is_ascii_alphanumeric() {
            return Some(i);
        }
    }
    line.match_indices("invoke ")
        .map(|(i, _)| i)
        .find(|&i| i == 0 || !line.as_bytes()[i - 1].is_ascii_alphanumeric())
}

fn is_alloca_instruction(line: &str) -> bool {
    // `%x = alloca ...`
    line.contains(" alloca ")
}

fn is_branch_instruction(line: &str) -> bool {
    // `br label ...` or `br i1 ...` or `switch ...`
    let trimmed = line.trim();
    trimmed.starts_with("br ") || trimmed.starts_with("switch ")
}

fn is_phi_instruction(line: &str) -> bool {
    // `%x = phi ...`
    line.contains(" phi ")
}

/// Parse all function definitions from an LLVM IR file.
/// Returns a list of FunctionIr.
pub(crate) fn parse_ir_functions(content: &str) -> Vec<FunctionIr> {
    let mut functions = Vec::new();
    let mut i = 0;
    let chars: Vec<char> = content.chars().collect();
    let len = chars.len();

    while i < len {
        // Look for `define` at the start of a line
        if (i == 0 || chars[i - 1] == '\n') && content[i..].starts_with("define ") {
            // Find end of this line
            let line_end = content[i..].find('\n').map(|n| i + n).unwrap_or(len);
            let line = &content[i..line_end];

            // Find the function name (@name)
            if let Some(name_start) = line.find('@') {
                let abs_name_start = i + name_start + 1;
                // Find end of name (opening paren)
                if let Some(paren_offset) = content[abs_name_start..].find('(') {
                    let name = &content[abs_name_start..abs_name_start + paren_offset];

                    // Find the LAST `{` on the define line (to skip struct return types like `{ i32, i32 }`)
                    if let Some(brace_offset) = line.rfind('{') {
                        let body_start = i + brace_offset + 1;
                        // Find matching closing brace (functions end with `}\n`)
                        if let Some(body_end) = find_closing_brace(content, body_start) {
                            let body = &content[body_start..body_end];
                            let raw = &content[i..body_end + 1];
                            functions.push(parse_function_body(name, body, raw));
                            i = body_end + 1;
                            continue;
                        }
                    }
                }
            }
        }
        // Advance to next line
        if let Some(nl) = content[i..].find('\n') {
            i += nl + 1;
        } else {
            break;
        }
    }

    functions
}

/// Find the closing `}` that matches the opening brace.
/// For LLVM IR, function bodies end with `}` at the start of a line.
fn find_closing_brace(content: &str, start: usize) -> Option<usize> {
    // In LLVM IR, the closing brace of a function is always at column 0
    for (i, _) in content[start..].match_indices('\n') {
        let next_line_start = start + i + 1;
        if content[next_line_start..].starts_with('}') {
            return Some(next_line_start);
        }
    }
    None
}
