import unittest

from tide import Reading, high_water, low_water, safe_hours, tidal_range

DAY = [Reading(0, 1.2), Reading(3, 3.8), Reading(6, 4.1), Reading(9, 1.9), Reading(12, 0.6)]


class TideTest(unittest.TestCase):
    def test_high_water(self):
        self.assertEqual(high_water(DAY).hour, 6)

    def test_low_water(self):
        self.assertEqual(low_water(DAY).hour, 12)

    def test_tidal_range(self):
        self.assertAlmostEqual(tidal_range(DAY), 3.5)

    def test_safe_hours(self):
        self.assertEqual(safe_hours(DAY, 2.0), [3, 6])


if __name__ == "__main__":
    unittest.main()
