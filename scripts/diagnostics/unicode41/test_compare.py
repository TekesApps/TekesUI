import copy
import unittest
from compare import summarize


def rows(control='sdk', value='原生A草稿保留', case='mixed'):
    base = {'control': control, 'declared_source': 'cua-typeText', 'case': case,
            'pid': 41, 'active': True, 'focused': True, 'actions': 0,
            'selection': [0, 0], 'marked': None, 'value': ''}
    ready = dict(base, seq=1, kind='ready', detail={})
    commit = dict(base, seq=2, kind='commit', detail={'text': value})
    result = dict(base, seq=3, kind='result', value=value, selection=[len(value.encode('utf-16-le')) // 2] * 2,
                  detail={'expected': '你好' if case == 'pinyin-first' else '原生A草稿保留',
                          'matches': True, 'no_marked': True, 'no_submit': True, 'ready': True})
    return [ready, commit, result]


class ComparisonTests(unittest.TestCase):
    def test_ascii_only_loss_is_failure_even_with_claimed_success(self):
        self.assertFalse(summarize(rows(value='A'), 'sdk', 'cua-typeText', 'mixed')['passed'])

    def test_duplicate_commit_visible_in_value_is_failure(self):
        self.assertFalse(summarize(rows(value='原生A草稿保留原生A草稿保留'), 'sdk', 'cua-typeText', 'mixed')['passed'])

    def test_nonempty_or_inactive_initial_editor_is_not_a_valid_baseline(self):
        for field, value in [('value', 'old'), ('active', False), ('focused', False), ('marked', [0, 1])]:
            r = rows(); r[0][field] = value
            with self.subTest(field=field), self.assertRaises(ValueError):
                summarize(r, 'sdk', 'cua-typeText', 'mixed')

    def test_unfinished_key_context_is_not_accepted(self):
        r = rows(); r.insert(1, dict(r[0], seq=2, kind='key_context', detail={'phase': 'start', 'context_id': 1, 'characters': '原'}))
        with self.assertRaises(ValueError):
            summarize(r, 'sdk', 'cua-typeText', 'mixed')

    def test_first_pinyin_commit_fails_even_if_final_value_is_correct(self):
        r = rows(value='你好', case='pinyin-first')
        self.assertFalse(summarize(r, 'sdk', 'cua-typeText', 'pinyin-first')['passed'])
        r[1]['kind'] = 'preedit'
        self.assertTrue(summarize(r, 'sdk', 'cua-typeText', 'pinyin-first')['passed'])

    def test_result_missing_or_submit_or_remaining_marked_fails(self):
        r = rows()
        with self.assertRaises(ValueError):
            summarize(r[:-1], 'sdk', 'cua-typeText', 'mixed')
        for field, value in [('actions', 1), ('marked', [0, 1])]:
            changed = copy.deepcopy(r); changed[-1][field] = value
            self.assertFalse(summarize(changed, 'sdk', 'cua-typeText', 'mixed')['passed'])

    def test_exact_pair_summary_retains_callback_order(self):
        result = summarize(rows(), 'sdk', 'cua-typeText', 'mixed')
        self.assertTrue(result['passed'])
        self.assertEqual(result['text_callback_order'], ['commit'])


if __name__ == '__main__':
    unittest.main()
