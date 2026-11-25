# SPDX-FileCopyrightText: 2026 Cubeyond
# SPDX-License-Identifier: AGPL-3.0-or-later

"""Replay the provider rejection seen on the first authenticated album listing."""

import io
import json
import unittest
from unittest.mock import patch
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
