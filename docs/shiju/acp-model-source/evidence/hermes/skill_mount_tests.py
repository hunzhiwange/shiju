"""Run against the actual reviewed Hermes package, never a fake scanner."""
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

from agent.prompt_builder import build_skills_system_prompt, clear_skills_system_prompt_cache
from tools.skills_tool import skill_view, _find_all_skills


class SessionSkillDirectoryTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.view = self.root / "view"
        self.home = self.root / "native"
        self.external = self.root / "external"
        for base, name in ((self.view, "selected"), (self.home / "skills", "user-decoy"), (self.external, "external-decoy")):
            skill = base / name
            skill.mkdir(parents=True)
            (skill / "SKILL.md").write_text(f"---\nname: {name}\ndescription: Metadata\n---\nBODY_{name}\n")
        (self.view / "selected/value.txt").write_text("attachment")
        (self.home / "config.yaml").write_text(json.dumps({"skills": {"external_dirs": [str(self.external)]}}))
        self.env = patch.dict(os.environ, {"HERMES_HOME": str(self.home), "SHIJU_HERMES_SKILL_ROOT": str(self.view)})
        self.env.start()
        self.addCleanup(self.env.stop)
        clear_skills_system_prompt_cache()

    def test_native_catalog_contains_only_selected_metadata(self):
        prompt = build_skills_system_prompt()
        self.assertIn("- selected: Metadata", prompt)
        self.assertNotIn("BODY_selected", prompt)
        self.assertNotIn("decoy", prompt)
        self.assertEqual([s["name"] for s in _find_all_skills()], ["selected"])
        self.assertFalse((self.home / ".skills_prompt_snapshot.json").exists())

    def test_native_activation_and_attachment_use_view(self):
        result = json.loads(skill_view("selected"))
        self.assertTrue(result["success"])
        self.assertEqual(result["skill_dir"], str(self.view / "selected"))
        self.assertIn("BODY_selected", str(result))
        self.assertIn("attachment", skill_view("selected", file_path="value.txt"))
        self.assertFalse(json.loads(skill_view("selected", file_path="../outside"))["success"])

    def test_unselected_and_plugin_skills_are_denied(self):
        for name in ("user-decoy", "external-decoy", "plugin:skill"):
            self.assertFalse(json.loads(skill_view(name))["success"])

    def test_invalid_root_does_not_fall_back_to_native_skills(self):
        for raw in ("relative", str(self.root / "missing")):
            os.environ["SHIJU_HERMES_SKILL_ROOT"] = raw
            with self.assertRaises(ValueError):
                build_skills_system_prompt()
            self.assertFalse(json.loads(skill_view("user-decoy"))["success"])

    def test_empty_selection_has_no_catalog_and_rejects_activation(self):
        os.environ["SHIJU_HERMES_SKILL_ROOT"] = ""
        self.assertEqual(build_skills_system_prompt(), "")
        self.assertEqual(_find_all_skills(), [])
        self.assertFalse(json.loads(skill_view("selected"))["success"])


if __name__ == "__main__":
    unittest.main()
