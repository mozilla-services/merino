# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at http://mozilla.org/MPL/2.0/.

import logging
import os
from random import choice, choices, randint
from typing import Any, Dict, List

import faker
import kinto_http
from client_info import DESKTOP_FIREFOX, LOCALES
from kinto import download_suggestions
from locust import HttpUser, events, task
from locust.clients import HttpSession
from locust.runners import MasterRunner
from models import ResponseContent

# TODO: Load logging level from environment variables
logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)

# See https://mozilla-services.github.io/merino/api.html#suggest
SUGGEST_API: str = "/api/v1/suggest"

# Optional. A comma-separated list of any experiments or rollouts that are
# affecting the client's Suggest experience
CLIENT_VARIANTS: str = ""

# Optional. A comma-separated list of providers to use for this request.
PROVIDERS: str = ""

# See RemoteSettingsGlobalSettings in
# https://github.com/mozilla-services/merino/blob/main/merino-settings/src/lib.rs
KINTO__SERVER_URL = os.environ["KINTO__SERVER_URL"]

# See default values in RemoteSettingsConfig in
# https://github.com/mozilla-services/merino/blob/main/merino-settings/src/providers.rs
KINTO__BUCKET = os.environ["KINTO__BUCKET"]
KINTO__COLLECTION = os.environ["KINTO__COLLECTION"]

# The number of random suggestions stored on each worker
RS_SUGGESTIONS_COUNT: int = 100

# This will be populated on each worker and
RS_SUGGESTIONS: List[Dict] = []


@events.test_start.add_listener
def on_locust_test_start(environment, **kwargs):
    """Download suggestions from Kinto and store random suggestions on workers."""

    if not isinstance(environment.runner, MasterRunner):
        return

    kinto_client = kinto_http.Client(
        server_url=KINTO__SERVER_URL,
        bucket=KINTO__BUCKET,
        collection=KINTO__COLLECTION,
    )

    kinto_suggestions = download_suggestions(kinto_client)

    suggestions = [suggestion.dict() for suggestion in kinto_suggestions.values()]

    logger.info("download_suggestions: Downloaded %d suggestions", len(suggestions))

    for worker in environment.runner.clients:
        environment.runner.send_message(
            "store_suggestions",
            data=choices(suggestions, k=RS_SUGGESTIONS_COUNT),
            client_id=worker,
        )


def store_suggestions(environment, msg, **kwargs):
    """Modify the module scoped list with suggestions in-place."""
    logger.info("store_suggestions: Storing %d suggestions", len(msg.data))
    RS_SUGGESTIONS[:] = msg.data


@events.init.add_listener
def on_locust_init(environment, **kwargs):
    """Register a message on worker nodes."""
    if not isinstance(environment.runner, MasterRunner):
        environment.runner.register_message("store_suggestions", store_suggestions)


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

    def on_start(self):
        # Create a Faker instance for generating random suggest queries
        self.faker = faker.Faker(locale="en-US", providers=["faker.providers.lorem"])

        # By this time suggestions are expected to be stored on the worker
        logger.debug(
            "user will be sending queries for suggestions: %s",
            [suggestion["title"] for suggestion in RS_SUGGESTIONS],
        )

        return super().on_start()

    @task(weight=10)
    def rs_suggestions(self) -> None:
        """Send multiple requests for Remote Settings queries."""

        suggestion = choice(RS_SUGGESTIONS)

        for query in suggestion["keywords"]:
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
