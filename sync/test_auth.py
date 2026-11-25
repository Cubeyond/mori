# SPDX-FileCopyrightText: 2026 Cubeyond
# SPDX-License-Identifier: AGPL-3.0-or-later

"""Replay the provider rejection seen on the first authenticated album listing."""

import io
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import Mock, patch
from urllib.error import HTTPError

import icloud


class AlbumAuthenticationTests(unittest.TestCase):
    def test_unauthorized_album_listing_without_web_token_requires_web_access(self):
        self._expect_album_rejection("X-APPLE-WEBAUTH-USER=fixture", "web_access")

    def test_unauthorized_album_listing_with_web_token_requires_reconnection(self):
        self._expect_album_rejection("X-APPLE-WEBAUTH-TOKEN=fixture", "auth")

    def _expect_album_rejection(self, cookies, expected):
        def provider_error(*_args, **_kwargs):
            detail = json.dumps({"error": "sign in failed: 401: {\"errorCode\":\"fixture\"}"})
            raise HTTPError("http://127.0.0.1", 500, "rclone fixture", {}, io.BytesIO(detail.encode()))

        with patch.object(icloud.Remote, "__init__", return_value=None):
            remote = icloud.Remote("photos")
        remote.port = 1
        remote.authorization = "fixture"
        remote.has_web_token = "X-APPLE-WEBAUTH-TOKEN=" in cookies
        with patch.object(icloud, "urlopen", side_effect=provider_error):
            with self.assertRaisesRegex(icloud.Failure, f"^{expected}$"):
                icloud.list_remote(remote, "", {"dirsOnly": True})


class RegionalAuthenticationTests(unittest.TestCase):
    def test_china_is_passed_to_rclone_before_sending_a_verification_code(self):
        connector = icloud.Connector()
        with patch.object(icloud, "Remote") as factory:
            remote = factory.return_value
            remote.call.return_value = {"State": "2fa_do", "Option": {"Name": "config_2fa"}}
            challenge = connector.authenticate({
                "op": "start", "challenge": "fixture", "account": "tester@example.invalid",
                "password": "fixture", "region": "china",
            })
        self.assertEqual(challenge["kind"], "code")
        self.assertEqual(remote.call.call_args.args[1]["parameters"]["region"], "china")

    def test_discovered_china_region_survives_binding_restart_and_cookie_rotation(self):
        connector = icloud.Connector()
        verified = Mock()
        verified.call.return_value = {
            "cookies": "fixture-cookie", "trust_token": "fixture-token", "region": "china",
            "password": "fixture-password",
        }
        connector.pending = {"account": "tester@example.invalid", "remote": verified}
        result = connector.challenge_result({"State": ""})
        self.assertEqual(result["session"], {
            "cookies": "fixture-cookie", "trust_token": "fixture-token", "region": "china",
        })
        verified.close.assert_called_once()

        restarted = icloud.Connector()
        with tempfile.TemporaryDirectory() as folder:
            with patch.object(icloud, "STAGE", Path(folder)), patch.object(icloud, "Remote") as factory:
                remote = restarted.reader({
                    "generation": "fixture", "account": result["account"], "session": result["session"],
                })
        self.assertEqual(factory.call_args.args[1]["region"], "china")
        remote.call.return_value = {**result["session"], "cookies": "renewed"}
        self.assertEqual(restarted.response([])["session"], {
            "cookies": "renewed", "trust_token": "fixture-token", "region": "china",
        })
