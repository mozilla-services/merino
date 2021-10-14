# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at http://mozilla.org/MPL/2.0/.


from typing import List

import pytest
import requests
from models import ResponseContent, Step

# A set of model fields that we do not want to include in the Python dict representation
EXCLUDED_FIELDS = {"request_id"}


@pytest.fixture(name="merino_url")
def fixture_merino_url(request):
    """Read the merino URL from the pytest config."""
    return request.config.option.merino_url


def test_merino(merino_url: str, steps: List[Step]):
    """Test for requesting suggestions from Merino."""

    for step in steps:
        # Each step in a test scenario consists of a request and a response.
        # Use the parameters to perform the request and verify the response.

        method = step.request.method
        url = f"{merino_url}{step.request.path}"
        headers = {header.name: header.value for header in step.request.headers}

        r = requests.request(method, url, headers=headers)

        error_message = (
            f"Expected status code {step.response.status_code},\n"
            f"but the status code in the response from Merino is {r.status_code}.\n"
            f"The response content is '{r.text}'."
        )

        assert r.status_code == step.response.status_code, error_message

        if r.status_code == 200:
            # If the response status code is 200 OK, create a pydantic model
            # instance for validating the response content from Merino. Then
            # generate a dict representation of the model and compare it with
            # the expected response content for this step in the test scenario.
            merino_response_content = ResponseContent(**r.json()).dict(
                exclude=EXCLUDED_FIELDS,
            )
            step_response_content = step.response.content.dict(
                exclude=EXCLUDED_FIELDS,
            )
            assert merino_response_content == step_response_content

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
