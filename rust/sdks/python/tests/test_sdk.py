"""Tests for the AmanClaw Python SDK."""

import json
import sys
import os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))

from amanclaw_sdk.types import SkillMetadata, SkillInput, SkillResult


def test_skill_metadata():
    meta = SkillMetadata(name="test", description="A test skill")
    d = meta.to_dict()
    assert d["name"] == "test"
    assert d["description"] == "A test skill"
    assert d["timeout_ms"] == 10000
    assert d["version"] == "0.1.0"


def test_skill_input_parse_args():
    inp = SkillInput(
        name="test",
        args='{"query": "hello"}',
        user_id="u1",
        platform="telegram",
    )
    args = inp.parse_args()
    assert args["query"] == "hello"


def test_skill_input_parse_invalid_args():
    inp = SkillInput(name="test", args="not json", user_id="u1", platform="test")
    args = inp.parse_args()
    assert args == {}


def test_skill_input_from_dict():
    data = {
        "name": "echo",
        "args": '{"text": "hi"}',
        "user_id": "user123",
        "platform": "slack",
    }
    inp = SkillInput.from_dict(data)
    assert inp.name == "echo"
    assert inp.platform == "slack"


def test_skill_result_ok():
    r = SkillResult.ok("success!")
    assert r.success is True
    assert r.output == "success!"
    assert r.error is None
    d = r.to_dict()
    assert d["success"] is True


def test_skill_result_err():
    r = SkillResult.err("something broke")
    assert r.success is False
    assert r.output == ""
    assert r.error == "something broke"


def test_plugin_decorator():
    from amanclaw_sdk import plugin

    @plugin(
        name="test_skill",
        description="Test",
        parameters={"type": "object", "properties": {}},
    )
    def my_skill(input: SkillInput) -> SkillResult:
        return SkillResult.ok("worked")

    assert my_skill.metadata.name == "test_skill"
    assert callable(my_skill.run)

    # Test direct execution
    inp = SkillInput(name="test_skill", args="{}", user_id="u1", platform="test")
    result = my_skill(inp)
    assert result.success is True
    assert result.output == "worked"


if __name__ == "__main__":
    test_skill_metadata()
    test_skill_input_parse_args()
    test_skill_input_parse_invalid_args()
    test_skill_input_from_dict()
    test_skill_result_ok()
    test_skill_result_err()
    test_plugin_decorator()
    print(f"All {7} tests passed!")
