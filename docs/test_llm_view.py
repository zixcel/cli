import unittest
from llm_view import gate, project, select


class ViewTests(unittest.TestCase):
    def test_projection_uses_only_registered_ids_and_keywords(self):
        metadata = {'schema': 'zix-capability-index/v1', 'scope': 'providers',
                    'providers': [{'namespace': 'github', 'topics': ['REPOSITORY'],
                                   'unrelated': 'SYNTHETIC_PRIVATE'}]}
        view = project(metadata)
        self.assertEqual(view['text'], 'github: REPOSITORY')
        self.assertNotIn('SYNTHETIC_PRIVATE', str(view))
        self.assertEqual(select(view, {'choice': 'github'}, ['github']), 'github')
        with self.assertRaises(ValueError):
            select(view, {'choice': 'github; arbitrary-command'}, ['github'])

    def test_registered_but_unapproved_choice_is_rejected(self):
        with self.assertRaises(ValueError):
            select({'choices': ['github'], 'gate': {'state': 'inspect_only'}}, {'choice': 'github'}, [])

    def test_missing_context_and_denial_are_not_model_decisions(self):
        self.assertEqual(gate({'missing_arguments': ['mailbox']})['next'], 'collect_input')
        self.assertEqual(gate({'diagnostic': {'state': 'NeedsResolution',
                                             'reason': 'authorization'}}),
                         {'state': 'blocked', 'next': 'review_access'})

    def test_unknown_write_requires_readback_and_other_errors_stop(self):
        self.assertEqual(gate({'diagnostic': {'state': 'NeedsResolution',
                                             'reason': 'outcome_unknown'}})['next'], 'readback')
        self.assertEqual(gate({'error': 'auth_not_registered'})['state'], 'blocked')

    def test_a_blocked_view_rejects_even_a_registered_choice(self):
        with self.assertRaises(ValueError):
            select({'choices': ['github'], 'gate': {'state': 'blocked'}}, {'choice': 'github'}, ['github'])


if __name__ == '__main__':
    unittest.main()
