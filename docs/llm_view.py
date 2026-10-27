#!/usr/bin/env python3
"""Project Zix metadata for small-context callers; never execute model output."""
import hashlib
import json
import re
import sys


def gate(value):
    """A model cannot override unresolved context or diagnostic stops."""
    if value.get('missing_arguments'):
        return {'state': 'blocked', 'next': 'collect_input'}
    diagnostic = value.get('diagnostic', {})
    if value.get('error') or diagnostic.get('state') == 'NeedsResolution':
        next_step = {'context': 'collect_input', 'authentication': 'inspect_authentication',
                     'authorization': 'review_access', 'outcome_unknown': 'readback',
                     'incomplete': 'inspect_partial_evidence'}.get(diagnostic.get('reason'), 'inspect_definition')
        return {'state': 'blocked', 'next': next_step}
    return {'state': 'inspect_only', 'next': 'verify_inputs_and_task_authorization'}


def project(value):
    """Keep the full response outside the prompt for provenance and validation."""
    if not isinstance(value, dict):
        raise ValueError('invalid_metadata')
    schema = value.get('schema')
    choices, lines = [], []
    if schema == 'zix-capability-index/v1':
        for provider in value['providers']:
            if value['scope'] == 'providers':
                choices.append(provider['namespace'])
                lines.append(provider['namespace'] + ': ' + ', '.join(provider['topics']))
            else:
                for op in provider['operations']:
                    choices.append(op['id'])
                    lines.append(op['id'] + ': ' + op['summary'].replace('_', ' '))
    elif schema == 'zix-operation-brief/v1':
        lines = [value['operation'] + ': ' + value['intent'].replace('_', ' '),
                 'effect: ' + value['effect'],
                 'missing_arguments: ' + ', '.join(value['missing_arguments'])]
        lines.extend(r['kind'] + ' ' + r['subject'] + ': ' + r['value'] for r in value['requirements'])
    elif value.get('diagnostic', {}).get('schema') in ('zix-diagnostic/v1', 'zix-local-diagnostic/v1'):
        lines = [value['diagnostic']['summary']]
    else:
        raise ValueError('unsupported_metadata')
    if len(choices) > 32 or any(not re.fullmatch(r'[a-z0-9._-]{1,100}', c) for c in choices):
        raise ValueError('invalid_choice_set')
    return {'schema': 'zix-llm-view/v1', 'choices': choices, 'text': '\n'.join(lines),
            'gate': gate(value), 'authority': 'none', 'model_output_executed': False,
            'source_sha256': hashlib.sha256(json.dumps(value, sort_keys=True).encode()).hexdigest()}


def select(view, answer, allowed_choices):
    if set(answer) != {'choice'} or answer['choice'] not in view['choices'] or answer['choice'] not in allowed_choices:
        raise ValueError('unregistered_choice')
    if view['gate']['state'] != 'inspect_only':
        raise ValueError('unresolved_precondition')
    return answer['choice']


def select_id(view, text, allowed_choices):
    """A decoder may return one ID; the caller supplies the JSON envelope."""
    if not isinstance(text, str) or len(text.encode()) > 128:
        raise ValueError('invalid_model_output')
    return select(view, {'choice': text.strip()}, allowed_choices)


def main():
    try:
        raw = sys.stdin.buffer.read(65537)
        if len(raw) > 65536:
            raise ValueError('metadata_limit')
        print(json.dumps(project(json.loads(raw))))
    except (ValueError, KeyError, TypeError):
        print('{"error":"invalid_metadata","authority":"none"}')
        return 2
    return 0


if __name__ == '__main__':
    sys.exit(main())
