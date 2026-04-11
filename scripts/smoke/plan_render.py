from __future__ import annotations

import json

from .models import SmokePlan


def render_text(plan: SmokePlan) -> str:
    lines = list(plan.summary_lines)
    lines.append("Inputs:")
    lines.extend(f"- {item}" for item in plan.inputs)
    lines.append("Steps:")
    lines.extend(f"- {item}" for item in plan.steps)
    lines.append("Commands:")
    for command in plan.commands:
        lines.append(f"- {command.name}: {' '.join(command.argv)}")
    if plan.helpers:
        lines.append("Helpers:")
        for helper in plan.helpers:
            lines.append(f"- {helper.name}: {' '.join(helper.command.argv)}")
    if plan.probe_paths:
        lines.append("Probe paths:")
        lines.extend(f"- /boot/{item}" for item in plan.probe_paths)
    if plan.structured_notes:
        lines.append("Notes:")
        lines.extend(f"- {item}" for item in plan.structured_notes)
    return "\n".join(lines) + "\n"


def render_json(plan: SmokePlan) -> str:
    return json.dumps(plan.to_json_dict(), indent=2, sort_keys=True) + "\n"
