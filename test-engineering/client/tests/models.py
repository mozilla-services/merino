# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at http://mozilla.org/MPL/2.0/.

from typing import Any, List, Optional, Union

from pydantic import BaseModel, Extra


class Header(BaseModel):
    """Class that holds information about a HTTP header."""

    name: str
    value: str


class Request(BaseModel):
    """Class that holds information about a HTTP request to Merino."""

    method: str
    path: str
    headers: List[Header] = []


class Suggestion(BaseModel, extra=Extra.allow):
    """Class that holds information about a Suggestion returned by Merino."""

    block_id: int
    full_keyword: str
    title: str
    url: str
    impression_url: str
    click_url: str
    provider: str
    is_sponsored: bool
    icon: str
    score: float
    advertiser: Optional[str]  # A deprecated alias of `provider`


class Suggestions(BaseModel):
    """Class that contains a list of Suggestions returned by Merino."""

    suggestions: List[Suggestion]


class Response(BaseModel):
    """Class that holds information about a HTTP response from Merino."""

    status_code: int
    content: Union[Suggestions, Any]
    headers: List[Header] = []


class Step(BaseModel):
    """Class that holds information about a step in a test scenario."""

    request: Request
    response: Response


class Scenario(BaseModel):
    """Class that holds information about a specific test scenario."""

    name: str
    description: str
    steps: List[Step]
