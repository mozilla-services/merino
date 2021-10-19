# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at http://mozilla.org/MPL/2.0/.

from random import choice
from typing import Any, Dict

from client_info import DESKTOP_FIREFOX, LOCALES
from locust import HttpUser, task
from locust.clients import HttpSession
from models import ResponseContent

# See https://mozilla-services.github.io/merino/api.html#suggest
SUGGEST_API: str = "/api/v1/suggest"

# Optional. A comma-separated list of any experiments or rollouts that are
# affecting the client's Suggest experience
CLIENT_VARIANTS: str = ""

# Optional. A comma-separated list of providers to use for this request.
PROVIDERS: str = ""


def request_suggestions_for_word(client: HttpSession, word: str) -> None:
    """Request suggestions for slices of the given word."""

    for query in [word[: i + 1] for i in range(len(word))]:
        request_suggestions(client, query)


def request_suggestions(client: HttpSession, query: str) -> None:
    """Request suggestions from Merino for the given query string."""

    params: Dict[str, Any] = {"q": query}

    if CLIENT_VARIANTS:
        params = {**params, "client_variants": CLIENT_VARIANTS}

    if PROVIDERS:
        params = {**params, "providers": PROVIDERS}

    headers: Dict[str, str] = {
        "Accept-Language": choice(LOCALES),
        "User-Agent": choice(DESKTOP_FIREFOX),
    }

    with client.get(
        url=SUGGEST_API, params=params, headers=headers, catch_response=True
    ) as response:
        # This contextmanager returns a response that provides the ability to
        # manually control if an HTTP request should be marked as successful or
        # a failure in Locust's statistics
        if response.status_code != 200:
            response.failure(f"{response.status_code=}, expected 200, {response.text=}")
            return

        # Create a pydantic model instance for validating the response content
        # from Merino. This will raise an Exception if the response is missing
        # fields which will be reported as a failure in Locust's statistics.
        ResponseContent(**response.json())


class MerinoUser(HttpUser):
    """User that sends requests to the Merino API."""

    @task(weight=1)
    def suggest_apple(self) -> None:
        """Send multiple requests for the word apple."""
        request_suggestions_for_word(self.client, "apple")

    @task(weight=1)
    def suggest_banana(self) -> None:
        """Send multiple requests for the word banana."""
        request_suggestions_for_word(self.client, "banana")

    @task(weight=1)
    def suggest_cherry(self) -> None:
        """Send multiple requests for the word cherry."""
        request_suggestions_for_word(self.client, "cherry")
