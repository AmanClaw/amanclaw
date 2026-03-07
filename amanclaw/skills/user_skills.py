"""
User Skills — HTTP-only sandboxed execution for user-created API integrations.
"""

import re
import json
import logging
import requests
from amanclaw.memory import Memory

logger = logging.getLogger("amanclaw.skills.user_skills")

# Max response size to prevent memory issues
MAX_RESPONSE_SIZE = 1_000_000  # 1MB
REQUEST_TIMEOUT = 10  # seconds


class UserSkillManager:
    """Manages user-created skills: tool definitions + sandboxed HTTP execution."""

    def __init__(self, memory: Memory):
        self.memory = memory

    def get_tool_definitions(self, user_id: str) -> list[dict]:
        """Get user skills as LLM tool definitions for a specific user."""
        skills = self.memory.get_user_skills(user_id)
        tools = []
        for s in skills:
            params = json.loads(s["parameters"]) if isinstance(s["parameters"], str) else s["parameters"]
            tools.append({
                "name": f"uskill_{s['name']}",
                "description": s["description"],
                "input_schema": {
                    "type": "object",
                    "properties": params,
                    "required": [k for k, v in params.items() if not v.get("optional", False)],
                },
            })
        return tools

    def has_skill(self, tool_name: str) -> bool:
        """Check if a tool name is a user skill."""
        return tool_name.startswith("uskill_")

    def execute(self, tool_name: str, tool_input: dict, user_id: str) -> str:
        """Execute a user skill by making the configured HTTP request."""
        skill_name = tool_name.replace("uskill_", "", 1)
        skill = self.memory.get_user_skill_by_name(skill_name, user_id)

        if not skill:
            return f"Error: User skill '{skill_name}' not found"

        try:
            return self._run_http_request(skill, tool_input)
        except requests.Timeout:
            return f"Skill '{skill_name}' timed out after {REQUEST_TIMEOUT}s"
        except requests.RequestException as e:
            logger.error(f"User skill '{skill_name}' HTTP error: {e}")
            return f"Skill '{skill_name}' failed: {e}"
        except Exception as e:
            logger.error(f"User skill '{skill_name}' error: {e}")
            return f"Skill '{skill_name}' error: {type(e).__name__}: {e}"

    def _run_http_request(self, skill: dict, params: dict) -> str:
        """Execute the HTTP request defined by a user skill."""
        # Substitute parameters into URL template
        url = self._substitute(skill["url_template"], params)

        # Build headers
        headers_raw = json.loads(skill["headers"]) if isinstance(skill["headers"], str) else (skill["headers"] or {})
        headers = {k: self._substitute(v, params) for k, v in headers_raw.items()}

        # Substitute API key if present
        if skill.get("api_key_encrypted"):
            api_key = skill["api_key_encrypted"]  # TODO: decrypt in future
            headers = {k: v.replace("{api_key}", api_key) for k, v in headers.items()}
            url = url.replace("{api_key}", api_key)

        # Build query params
        qp_raw = json.loads(skill["query_params"]) if isinstance(skill["query_params"], str) else (skill["query_params"] or {})
        query_params = {k: self._substitute(v, params) for k, v in qp_raw.items()}

        # Build body for POST
        body = None
        method = (skill.get("method") or "GET").upper()
        if method == "POST" and skill.get("body_template"):
            body_raw = json.loads(skill["body_template"]) if isinstance(skill["body_template"], str) else skill["body_template"]
            body = json.loads(self._substitute(json.dumps(body_raw), params))

        # Make request
        logger.info(f"User skill HTTP {method} {url}")
        resp = requests.request(
            method=method,
            url=url,
            headers=headers,
            params=query_params if query_params else None,
            json=body,
            timeout=REQUEST_TIMEOUT,
        )
        resp.raise_for_status()

        # Size check
        if len(resp.content) > MAX_RESPONSE_SIZE:
            return "Error: Response too large (>1MB)"

        # Parse response
        try:
            data = resp.json()
        except ValueError:
            return resp.text[:2000]

        # Apply response mapping if defined
        response_mapping = skill.get("response_mapping")
        if response_mapping:
            mapping = json.loads(response_mapping) if isinstance(response_mapping, str) else response_mapping
            if mapping:
                extracted = {}
                for key, path in mapping.items():
                    extracted[key] = self._extract_jsonpath(data, path)
                data = extracted

        # Apply response format template if defined
        response_format = skill.get("response_format")
        if response_format:
            try:
                return response_format.format(**data) if isinstance(data, dict) else str(data)
            except (KeyError, IndexError):
                pass

        # Default: return pretty JSON
        return json.dumps(data, indent=2, ensure_ascii=False)[:3000]

    @staticmethod
    def _substitute(template: str, params: dict) -> str:
        """Replace {param_name} placeholders with actual values."""
        result = template
        for key, value in params.items():
            result = result.replace(f"{{{key}}}", str(value))
        return result

    @staticmethod
    def _extract_jsonpath(data: dict, path: str):
        """Simple JSONPath-like extraction: $.field.nested[0].value"""
        path = path.lstrip("$.")
        current = data
        for part in re.split(r'\.|\[|\]', path):
            if not part:
                continue
            if part.isdigit():
                try:
                    current = current[int(part)]
                except (IndexError, KeyError, TypeError):
                    return None
            else:
                try:
                    current = current[part]
                except (KeyError, TypeError):
                    return None
        return current
