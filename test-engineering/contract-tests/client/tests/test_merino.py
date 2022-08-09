# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at http://mozilla.org/MPL/2.0/.


from typing import Dict, List, Set, Tuple

import pytest
import time
import requests
from models import ResponseContent, Step, Suggestion

# We need to exclude the following fields on the response level:
# The request ID is dynamic in nature and the value cannot be validated here.
# The suggestions are validated separately in a different step.
CONTENT_EXCLUDE: Set[str] = {"request_id", "suggestions"}

# We need to exclude the following field on the suggestion level:
# The icon URL for RS suggestions is dynamic in nature and handed to Merino by
# Kinto. We validate that in a seperate step.
SUGGESTION_EXCLUDE: Set[str] = {"icon"}


@pytest.fixture(scope="session", name="merino_url")
def fixture_merino_url(request) -> str:
    """Read the merino URL from the pytest config."""
    return request.config.option.merino_url


def suggestion_id(suggestion: Suggestion) -> Tuple:
    """Return the values for the fields that identify a suggestion."""
    return suggestion.provider, suggestion.block_id


def assert_200_response(
    *,
    step_content: ResponseContent,
    merino_content: ResponseContent,
    kinto_icon_urls: Dict[str, str],
) -> None:
    """Check that the content for a 200 OK response is what we expect."""

    expected_content_dict = step_content.dict(exclude=CONTENT_EXCLUDE)
    merino_content_dict = merino_content.dict(exclude=CONTENT_EXCLUDE)
    assert expected_content_dict == merino_content_dict

    # The order of suggestions in Merino's response is not guaranteed.
    # Sort them by ('provider', 'block_id') before validating them.
    sorted_merino_suggestions = [
        suggestion.dict(exclude=SUGGESTION_EXCLUDE)
        for suggestion in sorted(merino_content.suggestions, key=suggestion_id)
    ]
    sorted_expected_suggestions = [
        suggestion.dict(exclude=SUGGESTION_EXCLUDE)
        for suggestion in sorted(step_content.suggestions, key=suggestion_id)
    ]
    assert sorted_merino_suggestions == sorted_expected_suggestions

    # This is for selecting the right expected suggestion for a given Merino
    # suggestion based on the ('provider', 'block_id') fields.
    expected_suggestions_by_id = {
        suggestion_id(suggestion): suggestion for suggestion in step_content.suggestions
    }

    for suggestion in merino_content.suggestions:
        if "remote_settings" in suggestion.provider:
            # The icon URL is not static for RS suggestions
            assert suggestion.icon == kinto_icon_urls[suggestion.title]
            continue

        if "wiki_fruit" in suggestion.provider:
            # The icon URL is static for WikiFruit suggestions
            expected_suggestion = expected_suggestions_by_id[suggestion_id(
                suggestion)]
            assert suggestion.icon == expected_suggestion.icon
            continue


def test_merino(merino_url: str, steps: List[Step], kinto_icon_urls: Dict[str, str]):
    """Test for requesting suggestions from Merino."""

    for step in steps:
        # Each step in a test scenario consists of a request and a response.
        # Use the parameters to perform the request and verify the response.

        method = step.request.method
        url = f"{merino_url}{step.request.path}"
        headers = {header.name: header.value for header in step.request.headers}
        delay = step.request.delay

        # Process delay if defined in request model
        if delay > 0:
            time.sleep(delay)

        r = requests.request(method, url, headers=headers)

        error_message = (
            f"Expected status code {step.response.status_code},\n"
            f"but the status code in the response from Merino is {r.status_code}.\n"
            f"The response content is '{r.text}'."
        )

        assert r.status_code == step.response.status_code, error_message

        if r.status_code == 200:
            # If the response status code is 200 OK, use the
            # assert_200_response() helper function to validate the content of
            # the response from Merino. This includes creating a pydantic model
            # instance for checking the field types and comparing a dict
            # representation of the model instance with the expected response
            # content for this step in the test scenario.
            assert_200_response(
                step_content=step.response.content,
                merino_content=ResponseContent(**r.json()),
                kinto_icon_urls=kinto_icon_urls,
            )
            continue

        if r.status_code == 204:
            # If the response status code is 204 No Content, load the response content
            # as text and compare against the value in the response model.
            assert r.text == step.response.content
            continue

        # If the request to Merino was not successful, load the response
        # content into a Python dict and compare against the value in the
        # response model
        assert r.json() == step.response.content
