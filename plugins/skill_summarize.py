#!/usr/bin/env python3
"""Summarize text content. Works best when the LLM uses this for long inputs."""
from amanclaw_sdk import plugin, SkillInput, SkillResult

@plugin(
    name="summarize",
    description="Summarize long text into key points. Provide the text and desired length.",
    parameters={
        "type": "object",
        "properties": {
            "text": {"type": "string", "description": "Text to summarize"},
            "style": {"type": "string", "description": "Style: bullets, paragraph, tldr", "enum": ["bullets", "paragraph", "tldr"]}
        },
        "required": ["text"]
    }
)
def execute(inp: SkillInput) -> SkillResult:
    args = inp.parse_args()
    text = args.get("text", "")
    style = args.get("style", "bullets")

    if not text:
        return SkillResult.err("Please provide text to summarize.")

    # This skill returns the text back for the LLM to summarize
    # The LLM will use its own capabilities to create the summary
    word_count = len(text.split())
    return SkillResult.ok(
        f"[Summarize request — {style} style, {word_count} words]\n\n{text}"
    )

if __name__ == "__main__":
    execute.run()
