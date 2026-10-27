"""Synthetic date ranges and CSV artifacts in fresh isolated fixture containers."""
import json
from pathlib import Path
import unittest
from suite import diagnostics

ROOT = Path(__file__).resolve().parents[1]


class LogSummaryDateRangesCases(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.case, _ = diagnostics.load_case(ROOT / 'case.json')


def add_test(child_id, identity, expected):
    def test(self):
        child = next(c for c in self.case['children'] if c['id'] == child_id)
        files = (child['gold'] if identity == 'gold' else {} if identity == 'nop' else
                 next(n['files'] for n in child['negatives'] if n['id'] == identity))
        self.assertEqual(diagnostics.run_checker(self.case['image'], ROOT, child, files), expected)
    setattr(LogSummaryDateRangesCases, f'test_{child_id}_{identity.replace("-", "_")}', test)


for child in json.loads((ROOT / 'case.json').read_text())['children']:
    for identity, expected in [('gold', 0), ('nop', 1)] + [(n['id'], 1) for n in child['negatives']]:
        add_test(child['id'], identity, expected)



class ExpectedRowsMatchSyntheticInputs(unittest.TestCase):
    """Recalculate expectations independently of the artifact checker."""

    def expected(self, child):
        return json.loads((ROOT / 'oracle' / child / 'expected.json').read_text())['rows']

    def test_m1_calendar_selection(self):
        from datetime import date, timedelta
        current = date(2024, 3, 2)
        starts = [current, current - timedelta(days=6), current - timedelta(days=29),
                  current.replace(day=1), date.min]
        periods = ['today', 'last_7_days', 'last_30_days', 'month_to_date', 'total']
        rows = [['period', 'filename']]
        for period, start in zip(periods, starts):
            for path in sorted((ROOT / 'fixtures/m1/logs').iterdir()):
                day = date.fromisoformat(path.name[:10])
                if period == 'total' or start <= day <= current:
                    rows.append([period, path.name])
        self.assertEqual(rows, self.expected('m1'))

    def test_m2_severity_field_aggregation(self):
        from collections import Counter
        counts = Counter(line.split()[2][1:-1]
                         for path in (ROOT / 'fixtures/m2/logs').iterdir()
                         for line in path.read_text().splitlines())
        self.assertEqual([['severity', 'count']] + [[s, str(counts[s])]
                         for s in ['ERROR', 'WARNING', 'INFO']], self.expected('m2'))

    def test_m3_duplicate_and_date_boundaries(self):
        from collections import Counter
        rows = [['period', 'severity', 'count']]
        for period, start in [('today', '2024-03-01'), ('last_7_days', '2024-02-24'),
                              ('last_30_days', '2024-02-01'), ('month_to_date', '2024-03-01'),
                              ('total', '')]:
            counts = Counter()
            for path in (ROOT / 'fixtures/m3/logs').iterdir():
                if period == 'total' or start <= path.name[:10] <= '2024-03-01':
                    counts.update(line.split()[2][1:-1] for line in path.read_text().splitlines())
            rows += [[period, s, str(counts[s])] for s in ['ERROR', 'WARNING', 'INFO']]
        self.assertEqual(rows, self.expected('m3'))

    def test_m4_zero_filled_column_format(self):
        counts = json.loads((ROOT / 'fixtures/m4/counts.json').read_text())
        rows = [['period', 'severity', 'count']]
        for period in ['today', 'last_7_days', 'last_30_days', 'month_to_date', 'total']:
            rows += [[period, s, str(counts[period].get(s, 0))]
                     for s in ['ERROR', 'WARNING', 'INFO']]
        self.assertEqual(rows, self.expected('m4'))


if __name__ == '__main__':
    unittest.main()
