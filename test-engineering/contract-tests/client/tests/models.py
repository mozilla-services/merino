# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at http://mozilla.org/MPL/2.0/.

from enum import Enum
from typing import Any, List, Literal, Optional, Union
from uuid import UUID

from pydantic import BaseModel, Extra, Field


class Service(Enum):
    """Enum with service options."""

    KINTO: str = "kinto"
    MERINO: str = "merino"


class Header(BaseModel):
    """Class that holds information about a HTTP header."""

    name: str
    value: str


class Request(BaseModel):
    """Class that holds information about an HTTP request."""

    service: Service


class KintoRequest(Request):
    """Class that holds information about a Kinto HTTP request."""

    data_type: Literal["data", "offline-expansion-data"]
    filename: str


class MerinoRequest(Request):
    """Class that holds information about a Merino HTTP request."""

    method: str
    path: str
    headers: List[Header] = []
    # Delay is optional, providing time for data refresh
    delay: Optional[float] = None


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
    # Both impression_url and click_url are optional. They're absent for
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
    """Class that holds information about an HTTP response from Merino."""

    status_code: int
    content: Optional[Union[ResponseContent, Any]]


class Step(BaseModel):
    """Class that holds information about a step in a test scenario."""

    request: Union[KintoRequest, MerinoRequest]
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
    # Both impression_url and click_url are optional. They're absent for
    # Mozilla-provided Wikipedia suggestions.
    click_url: Optional[str]
    impression_url: Optional[str]
