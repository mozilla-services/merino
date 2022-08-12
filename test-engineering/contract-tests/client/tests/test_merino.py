# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at http://mozilla.org/MPL/2.0/.

import time
from typing import Callable, Dict, List, Set, Tuple

import pytest
import requests
from requests import Response as RequestsResponse

from kinto import KintoEnvironment, KintoRecord, upload_attachment, upload_icons
from models import ResponseContent, Service, Step, Suggestion

# We need to exclude the following fields on the response level:
# The request ID is dynamic in nature and the value cannot be validated here.
# The suggestions are validated separately in a different step.
CONTENT_EXCLUDE: Set[str] = {"request_id", "suggestions"}

# We need to exclude the following field on the suggestion level:
# The icon URL for RS suggestions is dynamic in nature and handed to Merino by
# Kinto. We validate that in a separate step.
SUGGESTION_EXCLUDE: Set[str] = {"icon"}


@pytest.fixture(scope="session", name="merino_url")
def fixture_merino_url(request) -> str:
    """Read the merino URL from the pytest config."""
    return request.config.option.merino_url


@pytest.fixture(scope="session", name="kinto_step")
def fixture_kinto_step(
    kinto_environment: KintoEnvironment, kinto_records: Dict[str, KintoRecord]
) -> Callable:
    """Define execution instructions for Kinto scenario step."""

    def kinto_step(step: Step) -> None:
        record: KintoRecord = kinto_records.get(step.request.filename)

        upload_attachment(kinto_environment, record, step.request.data_type)

        icon_ids: Set[str] = {
            suggestion.icon for suggestion in record.attachment.suggestions
        }
        upload_icons(kinto_environment, icon_ids)

    return kinto_step


@pytest.fixture(scope="session", name="merino_step")
def fixture_merino_step(merino_url: str, kinto_icon_urls: Dict[str, str]) -> Callable:
    """Define execution instructions for Merino scenario step."""

    def merino_step(step: Step) -> None:
        method: str = step.request.method
        url: str = f"{merino_url}{step.request.path}"
        headers: Dict[str, str] = {
            header.name: header.value for header in step.request.headers
        }

        response: RequestsResponse = requests.request(method, url, headers=headers)

        error_message: str = (
            f"The expected status code is {step.response.status_code},\n"
            f"but the status code in the response is {response.status_code}.\n"
            f"The response content is '{response.text}'."
        )

        assert response.status_code == step.response.status_code, error_message

        if response.status_code == 200:
            # If the response status code is 200 OK, use the
            # assert_200_response() helper function to validate the content of
            # the response from Merino. This includes creating a pydantic model
            # instance for checking the field types and comparing a dict
            # representation of the model instance with the expected response
            # content for this step in the test scenario.
            assert_200_response(
                step_content=step.response.content,
                merino_content=ResponseContent(**response.json()),
                kinto_icon_urls=kinto_icon_urls,
            )
            return

        if response.status_code == 204:
            # If the response status code is 204 No Content, load the response content
            # as text and compare against the value in the response model.
            assert response.text == step.response.content
            return

        # If the request to Merino was not successful, load the response
        # content into a Python dict and compare against the value in the
        # response model
        assert response.json() == step.response.content

    return merino_step


@pytest.fixture(scope="session", name="step_functions")
def fixture_step_functions(
    kinto_step: Callable, merino_step: Callable
) -> Dict[Service, Callable]:
    """Return a dict mapping from a service name to request function."""

    return {
        Service.KINTO: kinto_step,
        Service.MERINO: merino_step,
    }


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
            expected_suggestion = expected_suggestions_by_id[suggestion_id(suggestion)]
            assert suggestion.icon == expected_suggestion.icon
            continue


def test_merino(steps: List[Step], step_functions: Dict[Service, Callable]):
    """Test for requesting suggestions from Merino."""

    for step in steps:
        # Process delay if defined in request model
        if (delay := step.request.delay) is not None:
            time.sleep(delay)

        step_functions[step.request.service](step)
