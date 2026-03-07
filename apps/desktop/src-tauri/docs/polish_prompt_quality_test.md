# Polish Engine Prompt 效果验证测试

## 概述

`polish_prompt_quality_test.rs` 包含 3 个集成测试，用于验证 polish engine 的 prompt 是否能产生正确的输出效果。

## 测试用例

### 1. `test_filler_prompt_effectiveness` - 填充词移除效果测试

验证 filler 模板的 prompt 是否能正确：
- 移除填充词（um, uh, like, you know 等）
- 保持原始语言不变（英文输入→英文输出，中文输入→中文输出）
- 保留核心含义和关键信息
- 正确处理没有填充词的文本

**测试场景：**
- 英文带填充词：`"Um, I think we should, like, go there tomorrow, you know?"`
- 中文带填充词：`"嗯，我觉得，就是说，我们应该明天去那里"`
- 无填充词文本：`"The function returns the result immediately."`

### 2. `test_language_preservation_effectiveness` - 语言保留效果测试

**最关键的测试**，验证所有模板的 prompt 是否能保持输入语言不变，防止模型将文本翻译成其他语言。

**测试场景：**
- 对每个模板（filler, formal, concise）测试：
  - 中文输入 → 验证输出仍为中文
  - 日文输入 → 验证输出仍为日文

### 3. `test_template_specific_behavior` - 模板特定行为测试

验证每个模板的 prompt 是否能执行其特定任务：

- **Formal 模板**：将随意的文本转换为正式风格
  - 输入：`"Hey, can you check this out? It's pretty cool!"`
  - 预期：移除 "Hey"、"pretty cool" 等随意表达

- **Concise 模板**：使文本更简洁
  - 输入：`"I think that, in my opinion, we should probably consider going there tomorrow."`
  - 预期：输出比输入更短，但保留关键信息 "tomorrow"

- **Agent 模板**：格式化为结构化的 markdown
  - 输入：`"I need to like create a button that shows loading when clicked"`
  - 预期：添加 markdown 结构（##, -, * 等），保留 "button" 和 "loading" 关键词

## 运行测试

### 前提条件

1. **下载模型文件**：测试需要 `qwen3.5-0.8b` 模型文件
   ```bash
   # 模型文件应该位于：
   # ~/Library/Application Support/com.ariatype.app/models/qwen3.5-0.8b-q8_0.gguf
   ```

2. **确认模型存在**：
   ```bash
   ls ~/Library/Application\ Support/com.ariatype.app/models/
   ```

### 运行所有测试

```bash
cd apps/desktop/src-tauri
cargo test --test polish_prompt_quality_test -- --ignored --nocapture
```

### 运行单个测试

```bash
# 只测试填充词移除
cargo test --test polish_prompt_quality_test test_filler_prompt_effectiveness -- --ignored --nocapture

# 只测试语言保留
cargo test --test polish_prompt_quality_test test_language_preservation_effectiveness -- --ignored --nocapture

# 只测试模板特定行为
cargo test --test polish_prompt_quality_test test_template_specific_behavior -- --ignored --nocapture
```

## 测试标记说明

所有测试都标记为 `#[ignore]`，原因：
- 需要下载的模型文件（约 1GB）
- 需要实际运行推理（每个测试约 10-30 秒）
- 不适合在 CI/CD 中自动运行

## 预期输出示例

```
=== Test Case 1: English with fillers ===
Input:  Um, I think we should, like, go there tomorrow, you know?
Output: I think we should go there tomorrow.

=== Test Case 2: Chinese with fillers ===
Input:  嗯，我觉得，就是说，我们应该明天去那里
Output: 我觉得我们应该明天去那里

=== Test Case 3: Text with no fillers ===
Input:  The function returns the result immediately.
Output: The function returns the result immediately.
```

## 如何验证 Prompt 质量

### 成功标准

1. **语言保留**：输出语言必须与输入语言一致
2. **任务执行**：每个模板应该执行其特定任务（移除填充词/正式化/简洁化/结构化）
3. **信息保留**：关键信息不应丢失
4. **无幻觉**：不应添加原文中不存在的内容

### 失败情况

如果测试失败，可能的原因：
1. **Prompt 指令不够清晰**：模型没有理解任务
2. **语言保留指令不够强**：模型将文本翻译成了其他语言
3. **示例不够好**：Prompt 中的示例没有正确引导模型行为
4. **指令冲突**：Prompt 中的不同指令相互矛盾

### 调试方法

1. 查看实际输出，对比预期
2. 检查 `templates.rs` 中对应模板的 `system_prompt`
3. 调整 prompt 指令的强度和清晰度
4. 添加或改进 prompt 中的示例

## 与其他测试的关系

- **单元测试**（`src/polish_engine/*/tests`）：测试代码逻辑和数据结构
- **集成测试**（`tests/polish_engine_test.rs`）：测试模块集成和 API
- **Prompt 效果测试**（本文件）：测试实际推理效果和 prompt 质量

## 注意事项

1. 测试结果可能因模型版本而异
2. 不同的量化版本（q4_0, q8_0）可能产生略有不同的结果
3. 测试断言设计为宽松验证，允许合理的输出变化
4. 如果需要更严格的验证，可以添加更多断言条件
