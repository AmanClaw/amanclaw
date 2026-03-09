use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};

const CARGO_TOML: &str = "Cargo.toml";
const SRC_LIB_RS: &str = "src/lib.rs";
const SKILL_MANIFEST: &str = "amanclaw-skill.toml";

/// Scaffold a new skill project.
pub fn scaffold_skill(name: &str, lang: &str, output_dir: Option<&str>) -> Result<PathBuf> {
    match lang {
        "rust" => scaffold_rust_skill(name, output_dir),
        "python" => scaffold_python_skill(name, output_dir),
        other => bail!("Unsupported language: {}. Use 'rust' or 'python'.", other),
    }
}

/// Create a Rust WASM skill project.
pub fn scaffold_rust_skill(name: &str, output_dir: Option<&str>) -> Result<PathBuf> {
    let base = output_dir
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    let dir_name = ["skill-", name].concat();
    let project_dir = base.join(&dir_name);
    let src_dir = project_dir.join("src");

    std::fs::create_dir_all(&src_dir)
        .with_context(|| format!("Failed to create {}", src_dir.display()))?;

    // Cargo.toml
    let crate_name = ["amanclaw-skill-", name].concat();
    let cargo_toml = format!(
        "[package]\n\
         name = \"{crate_name}\"\n\
         version = \"0.1.0\"\n\
         edition = \"2024\"\n\
         \n\
         [lib]\n\
         crate-type = [\"cdylib\"]\n\
         \n\
         [dependencies]\n\
         amanclaw-plugin-sdk = {{ path = \"../../crates/amanclaw-plugin-sdk\" }}\n\
         serde_json = \"1\"\n"
    );
    write_file(&project_dir.join(CARGO_TOML), &cargo_toml)?;

    // src/lib.rs
    let snake_name = name.replace('-', "_");
    let lib_rs = format!(
        "//! AmanClaw skill: {name}\n\
         \n\
         use amanclaw_plugin_sdk::*;\n\
         \n\
         amanclaw_plugin!(\n\
         {INDENT}metadata: SkillMetadata {{\n\
         {INDENT}{INDENT}name: \"{snake_name}\".into(),\n\
         {INDENT}{INDENT}description: \"TODO: describe {name}\".into(),\n\
         {INDENT}{INDENT}timeout_ms: 10000,\n\
         {INDENT}{INDENT}version: \"0.1.0\".into(),\n\
         {INDENT}}},\n\
         {INDENT}parameters: r#\"{{\"type\":\"object\",\"properties\":{{\"query\":{{\"type\":\"string\",\"description\":\"Input query\"}}}},\"required\":[\"query\"]}}\"#,\n\
         {INDENT}execute: |input: SkillInput| -> SkillResult {{\n\
         {INDENT}{INDENT}let args: serde_json::Value = serde_json::from_str(&input.args)\n\
         {INDENT}{INDENT}{INDENT}.unwrap_or_default();\n\
         {INDENT}{INDENT}let query = args[\"query\"].as_str().unwrap_or(\"(none)\");\n\
         {INDENT}{INDENT}SkillResult::ok(format!(\"{snake_name}: {{query}}\"))\n\
         {INDENT}}}\n\
         );\n",
        INDENT = "    "
    );
    write_file(&project_dir.join(SRC_LIB_RS), &lib_rs)?;

    // amanclaw-skill.toml manifest
    let manifest = format!(
        "[skill]\n\
         name = \"{snake_name}\"\n\
         version = \"0.1.0\"\n\
         description = \"TODO: describe {name}\"\n\
         language = \"rust\"\n\
         \n\
         [permissions]\n\
         network = false\n\
         filesystem = false\n"
    );
    write_file(&project_dir.join(SKILL_MANIFEST), &manifest)?;

    Ok(project_dir)
}

/// Create a Python script skill.
pub fn scaffold_python_skill(name: &str, output_dir: Option<&str>) -> Result<PathBuf> {
    let base = output_dir
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    let snake_name = name.replace('-', "_");

    std::fs::create_dir_all(&base)
        .with_context(|| format!("Failed to create {}", base.display()))?;

    let py_filename = ["skill_", &snake_name, ".py"].concat();
    let toml_filename = ["skill_", &snake_name, ".toml"].concat();

    // Python skill file
    let py_content = format!(
        "#!/usr/bin/env python3\n\
         \"\"\"AmanClaw skill: {name}\n\
         \n\
         Communicates with the engine via JSON-RPC protocol over stdin/stdout.\n\
         \"\"\"\n\
         \n\
         import json\n\
         import sys\n\
         \n\
         \n\
         METADATA = {{\n\
         {I}\"name\": \"{snake_name}\",\n\
         {I}\"description\": \"TODO: describe {name}\",\n\
         {I}\"version\": \"0.1.0\",\n\
         {I}\"timeout_ms\": 10000,\n\
         }}\n\
         \n\
         PARAMETERS = {{\n\
         {I}\"type\": \"object\",\n\
         {I}\"properties\": {{\n\
         {I}{I}\"query\": {{\n\
         {I}{I}{I}\"type\": \"string\",\n\
         {I}{I}{I}\"description\": \"Input query\",\n\
         {I}{I}}}\n\
         {I}}},\n\
         {I}\"required\": [\"query\"],\n\
         }}\n\
         \n\
         \n\
         def execute(input_data: dict) -> dict:\n\
         {I}\"\"\"Handle an execute request.\"\"\"\n\
         {I}args = input_data.get(\"args\", \"{{}}\")\n\
         {I}try:\n\
         {I}{I}parsed = json.loads(args)\n\
         {I}except json.JSONDecodeError:\n\
         {I}{I}parsed = {{}}\n\
         \n\
         {I}query = parsed.get(\"query\", \"(none)\")\n\
         {I}return {{\n\
         {I}{I}\"success\": True,\n\
         {I}{I}\"output\": f\"{snake_name}: {{query}}\",\n\
         {I}{I}\"error\": None,\n\
         {I}}}\n\
         \n\
         \n\
         def main():\n\
         {I}\"\"\"Run the JSON-RPC protocol loop.\"\"\"\n\
         {I}for line in sys.stdin:\n\
         {I}{I}line = line.strip()\n\
         {I}{I}if not line:\n\
         {I}{I}{I}continue\n\
         \n\
         {I}{I}try:\n\
         {I}{I}{I}request = json.loads(line)\n\
         {I}{I}except json.JSONDecodeError:\n\
         {I}{I}{I}respond({{\"error\": \"Invalid JSON\"}})\n\
         {I}{I}{I}continue\n\
         \n\
         {I}{I}method = request.get(\"method\", \"\")\n\
         \n\
         {I}{I}if method == \"metadata\":\n\
         {I}{I}{I}respond(METADATA)\n\
         {I}{I}elif method == \"parameters\":\n\
         {I}{I}{I}respond(PARAMETERS)\n\
         {I}{I}elif method == \"execute\":\n\
         {I}{I}{I}input_data = request.get(\"input\", {{}})\n\
         {I}{I}{I}try:\n\
         {I}{I}{I}{I}result = execute(input_data)\n\
         {I}{I}{I}{I}respond(result)\n\
         {I}{I}{I}except Exception as e:\n\
         {I}{I}{I}{I}respond({{\"success\": False, \"output\": \"\", \"error\": str(e)}})\n\
         {I}{I}elif method == \"shutdown\":\n\
         {I}{I}{I}break\n\
         {I}{I}else:\n\
         {I}{I}{I}respond({{\"error\": f\"Unknown method: {{method}}\"}})\n\
         \n\
         \n\
         def respond(data: dict):\n\
         {I}\"\"\"Write a JSON response line to stdout.\"\"\"\n\
         {I}print(json.dumps(data), flush=True)\n\
         \n\
         \n\
         if __name__ == \"__main__\":\n\
         {I}main()\n",
        I = "    "
    );
    write_file(&base.join(&py_filename), &py_content)?;

    // TOML manifest
    let manifest = format!(
        "[skill]\n\
         name = \"{snake_name}\"\n\
         version = \"0.1.0\"\n\
         description = \"TODO: describe {name}\"\n\
         language = \"python\"\n\
         entry = \"{py_filename}\"\n\
         \n\
         [permissions]\n\
         network = false\n\
         filesystem = false\n"
    );
    write_file(&base.join(&toml_filename), &manifest)?;

    Ok(base.to_path_buf())
}

fn write_file(path: &Path, content: &str) -> Result<()> {
    std::fs::write(path, content)
        .with_context(|| format!("Failed to write {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scaffold_rust_skill() {
        let tmp = tempfile::TempDir::new().unwrap();
        let out = tmp.path().to_str().unwrap();

        let project = scaffold_rust_skill("greeting", Some(out)).unwrap();

        assert!(project.join(CARGO_TOML).exists());
        assert!(project.join(SRC_LIB_RS).exists());
        assert!(project.join(SKILL_MANIFEST).exists());

        let cargo = std::fs::read_to_string(project.join(CARGO_TOML)).unwrap();
        assert!(cargo.contains("amanclaw-skill-greeting"));
        assert!(cargo.contains("amanclaw-plugin-sdk"));
        assert!(cargo.contains("crate-type = [\"cdylib\"]"));

        let lib = std::fs::read_to_string(project.join(SRC_LIB_RS)).unwrap();
        assert!(lib.contains("amanclaw_plugin!"));
        assert!(lib.contains("name: \"greeting\""));

        let manifest = std::fs::read_to_string(project.join(SKILL_MANIFEST)).unwrap();
        assert!(manifest.contains("name = \"greeting\""));
        assert!(manifest.contains("language = \"rust\""));
    }

    #[test]
    fn test_scaffold_python_skill() {
        let tmp = tempfile::TempDir::new().unwrap();
        let out = tmp.path().to_str().unwrap();

        let _project = scaffold_python_skill("my-tool", Some(out)).unwrap();

        let py_filename = "skill_my_tool.py";
        let toml_filename = "skill_my_tool.toml";
        let py_path = tmp.path().join(py_filename);
        let toml_path = tmp.path().join(toml_filename);

        assert!(py_path.exists());
        assert!(toml_path.exists());

        let py = std::fs::read_to_string(&py_path).unwrap();
        assert!(py.contains("\"name\": \"my_tool\""));
        assert!(py.contains("def execute("));
        assert!(py.contains("def main()"));
        // Check JSON-RPC method handling
        assert!(py.contains("method == \"metadata\""));
        assert!(py.contains("method == \"execute\""));
        assert!(py.contains("method == \"shutdown\""));

        let manifest = std::fs::read_to_string(&toml_path).unwrap();
        assert!(manifest.contains("name = \"my_tool\""));
        assert!(manifest.contains("language = \"python\""));
        assert!(manifest.contains(py_filename));
    }

    #[test]
    fn test_scaffold_rust_hyphenated_name() {
        let tmp = tempfile::TempDir::new().unwrap();
        let out = tmp.path().to_str().unwrap();

        let project = scaffold_rust_skill("prayer-time", Some(out)).unwrap();

        let lib = std::fs::read_to_string(project.join(SRC_LIB_RS)).unwrap();
        assert!(lib.contains("name: \"prayer_time\""));
    }

    #[test]
    fn test_scaffold_unsupported_language() {
        let tmp = tempfile::TempDir::new().unwrap();
        let out = tmp.path().to_str().unwrap();

        let result = scaffold_skill("test", "go", Some(out));
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Unsupported language"));
    }
}
