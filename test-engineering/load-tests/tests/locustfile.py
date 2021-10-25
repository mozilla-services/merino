# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at http://mozilla.org/MPL/2.0/.

import os
from pathlib import Path
from random import choice, randint
from typing import Any, Dict, List, Tuple

from client_info import DESKTOP_FIREFOX, LOCALES
from faker import Faker
from locust import HttpUser, task
from locust.clients import HttpSession
from models import ResponseContent
from rs_queries import load_from_file

# See https://mozilla-services.github.io/merino/api.html#suggest
SUGGEST_API: str = "/api/v1/suggest"

# Optional. A comma-separated list of any experiments or rollouts that are
# affecting the client's Suggest experience
CLIENT_VARIANTS: str = ""

# Optional. A comma-separated list of providers to use for this request.
PROVIDERS: str = ""


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

    rs_query_groups: List[Tuple[str, ...]]

    def on_start(self):
        # This expects an InstantSuggest_Queries_*.json file from the source-data
        # dir in the quicksuggest-rs repo for the path in RS_QUERIES_FILE
        self.rs_query_groups = load_from_file(Path(os.environ["RS_QUERIES_FILE"]))

        # Create a Faker instance for generating random suggest queries
        self.faker = Faker(locale="en-US", providers=["faker.providers.lorem"])

        return super().on_start()

    @task(weight=10)
    def rs_suggestions(self) -> None:
        """Send multiple requests for known RS queries."""

        for query in choice(self.rs_query_groups):
            request_suggestions(self.client, query)

    @task(weight=90)
    def faker_suggestions(self) -> None:
        """Send multiple requests for random queries."""

        # This produces a query between 2 and 4 random words
        full_query = " ".join(self.faker.words(nb=randint(2, 4)))

        for query in [full_query[: i + 1] for i in range(len(full_query))]:
            # Send multiple requests for the entire query, but skip spaces
            if query.endswith(" "):
                continue

            request_suggestions(self.client, query)

    @task(weight=1)
    def wikifruit_suggestions(self) -> None:
        """Send multiple requests for random WikiFruit queries."""

        # These queries are supported by the WikiFruit provider
        for fruit in ("apple", "banana", "cherry"):
            request_suggestions(self.client, fruit)
