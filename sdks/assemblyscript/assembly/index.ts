// Example AmanClaw Plugin in AssemblyScript
//
// This template shows how to write a plugin that the AmanClaw engine
// can load as a WASM module. Edit the metadata, parameters, and execute
// function to create your own skill.

import { SkillMetadata, SkillInput, SkillResult, stringToPtr } from "./sdk";

// ============================================================
// CUSTOMIZE YOUR PLUGIN BELOW
// ============================================================

/** Return metadata describing this skill. */
function getMetadata(): SkillMetadata {
  const meta = new SkillMetadata();
  meta.name = "hello_world";
  meta.description = "A hello world skill written in AssemblyScript";
  meta.timeout_ms = 10000;
  meta.version = "0.1.0";
  return meta;
}

/** Return JSON schema for the skill's parameters. */
function getParametersSchema(): string {
  return '{"type":"object","properties":{"name":{"type":"string","description":"Name to greet"}},"required":["name"]}';
}

/** Execute the skill with the given input. */
function executeSkill(input: SkillInput): SkillResult {
  // Parse the JSON arguments
  const args = JSON.parse<Map<string, string>>(input.args);
  const name = args.has("name") ? args.get("name") : "World";

  return SkillResult.ok("Hello, " + name + "! (from AssemblyScript)");
}

// ============================================================
// ABI EXPORTS — DO NOT MODIFY BELOW
// ============================================================

// alloc and dealloc are provided by AssemblyScript's exported runtime
// (via --exportRuntime / "exportRuntime": true in asconfig.json)

/** Return a pointer to null-terminated JSON metadata. */
export function metadata(): usize {
  const meta = getMetadata();
  const json = JSON.stringify(meta);
  return stringToPtr(json);
}

/** Return a pointer to null-terminated JSON schema string. */
export function parameters(): usize {
  const schema = getParametersSchema();
  return stringToPtr(schema);
}

/**
 * Execute the skill.
 * @param ptr - pointer to JSON SkillInput in linear memory
 * @param len - length of the JSON bytes
 * @returns pointer to null-terminated JSON SkillResult
 */
export function execute(ptr: usize, len: i32): usize {
  // Read input JSON from memory
  const inputBytes = String.UTF8.decodeUnsafe(ptr, len);
  let input: SkillInput;

  try {
    input = JSON.parse<SkillInput>(inputBytes);
  } catch (e) {
    const errResult = SkillResult.err("Failed to parse input: " + (e as Error).message);
    return stringToPtr(JSON.stringify(errResult));
  }

  // Call the user's execute function
  const result = executeSkill(input);
  return stringToPtr(JSON.stringify(result));
}
