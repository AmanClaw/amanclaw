// AmanClaw Plugin SDK for AssemblyScript
// This module provides the types and ABI glue for writing plugins.

// --- Types ---

@json
export class SkillMetadata {
  name!: string;
  description!: string;
  timeout_ms!: u32;
  version!: string;
}

@json
export class SkillInput {
  name!: string;
  args!: string;
  user_id!: string;
  platform!: string;
}

@json
export class SkillResult {
  success!: boolean;
  output!: string;
  error!: string | null;

  static ok(output: string): SkillResult {
    const r = new SkillResult();
    r.success = true;
    r.output = output;
    r.error = null;
    return r;
  }

  static err(error: string): SkillResult {
    const r = new SkillResult();
    r.success = false;
    r.output = "";
    r.error = error;
    return r;
  }
}

// --- ABI Helpers ---

/**
 * Write a string to linear memory and return a pointer to a null-terminated copy.
 * The caller (host) reads until \0.
 */
export function stringToPtr(s: string): usize {
  const encoded = String.UTF8.encode(s, true); // true = null-terminated
  return changetype<usize>(encoded);
}
