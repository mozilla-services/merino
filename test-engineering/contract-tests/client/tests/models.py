# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at http://mozilla.org/MPL/2.0/.

from typing import Any, List, Optional, Union
from uuid import UUID

from pydantic import BaseModel, Extra, Field


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
    provider: str
    advertiser: str
    is_sponsored: bool
    score: float
    icon: Optional[str] = Field(...)
    # Both impression_url and click_url are optinal. They're absent for
    # Mozilla-provided Wikipedia suggestions.
    impression_url: Optional[str]
    click_url: Optional[str]


class ResponseContent(BaseModel):
    """Class that contains suggestions and variants returned by Merino."""

    suggestions: List[Suggestion] = Field(default_factory=list)
    client_variants: List[str] = Field(default_factory=list)
    server_variants: List[str] = Field(default_factory=list)
    request_id: Optional[UUID] = Field(...)


class Response(BaseModel):
    """Class that holds information about a HTTP response from Merino."""

    status_code: int
    content: Union[ResponseContent, Any]
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


class KintoSuggestion(BaseModel):
    """Class that holds information about a Suggestion in Kinto."""

    id: int
    url: str
    iab_category: str
    icon: str
    advertiser: str
    title: str
    keywords: List[str] = Field(default_factory=list)
    # Both impression_url and click_url are optinal. They're absent for
    # Mozilla-provided Wikipedia suggestions.
    click_url: Optional[str]
    impression_url: Optional[str]
