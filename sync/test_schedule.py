# SPDX-FileCopyrightText: 2026 Cubeyond
# SPDX-License-Identifier: AGPL-3.0-or-later

"""Regression cases for calendar dates in the worker's actual timezone database."""

import datetime as dt
import unittest

from icloud import Failure, next_sync


def utc(value):
    return int(dt.datetime.fromisoformat(value).timestamp())


class ScheduleTests(unittest.TestCase):
    def weekly(self, after, time="03:00", weekday="sunday", timezone="Asia/Shanghai"):
        return next_sync({"frequency": "weekly", "weekday": weekday, "time": time,
                          "timezone": timezone, "after": utc(after)})

    def test_weekly_uses_local_weekday_and_advances_at_the_boundary(self):
        self.assertEqual(self.weekly("2026-11-28T18:59:59+00:00"), utc("2026-11-28T19:00:00+00:00"))
        self.assertEqual(self.weekly("2026-11-28T19:00:00+00:00"), utc("2026-12-05T19:00:00+00:00"))
        self.assertEqual(self.weekly("2026-11-28T20:00:00+00:00", weekday="monday"),
                         utc("2026-11-29T19:00:00+00:00"))

    def test_weekly_skips_a_nonexistent_spring_forward_time(self):
        self.assertEqual(self.weekly("2026-03-07T12:00:00+00:00", "02:30", timezone="America/New_York"),
                         utc("2026-03-15T06:30:00+00:00"))

    def test_weekly_uses_the_first_occurrence_of_a_repeated_time(self):
        self.assertEqual(self.weekly("2026-11-01T04:00:00+00:00", "01:30", timezone="America/New_York"),
                         utc("2026-11-01T05:30:00+00:00"))
        self.assertEqual(self.weekly("2026-11-01T06:15:00+00:00", "01:30", timezone="America/New_York"),
                         utc("2026-11-08T06:30:00+00:00"))

    def test_invalid_weekday_is_rejected(self):
        with self.assertRaisesRegex(Failure, "schedule"):
            self.weekly("2026-11-28T18:59:59+00:00", weekday="weekend")
