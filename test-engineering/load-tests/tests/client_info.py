# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at http://mozilla.org/MPL/2.0/.

# See https://mozilla-services.github.io/merino/api.html#headers

from typing import List

# Examples for supported User-Agent header values
DESKTOP_FIREFOX: List[str] = [
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:10.0) Gecko/20100101 Firefox/90.0"
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:10.0) Gecko/20100101 Firefox/91.0"
    "Mozilla/5.0 (Windows NT 10.0; rv:10.0) Gecko/20100101 Firefox/91.0"
    "Mozilla/5.0 (X11; Linux x86_64; rv:90.0) Gecko/20100101 Firefox/91.0"
]

# Examples for supported Accept-Language header values
LOCALES: List[str] = ["en-US"]
