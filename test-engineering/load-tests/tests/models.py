# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at http://mozilla.org/MPL/2.0/.

from typing import List, Optional
from uuid import UUID

from pydantic import BaseModel, Extra, Field


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


class ResponseContent(BaseModel):
    """Class that contains suggestions and variants returned by Merino."""

    suggestions: List[Suggestion] = Field(default_factory=list)
    client_variants: List[str] = Field(default_factory=list)
    server_variants: List[str] = Field(default_factory=list)
    request_id: Optional[UUID] = Field(...)
