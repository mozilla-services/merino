# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at http://mozilla.org/MPL/2.0/.


class KintoError(Exception):
    """Error specific to Kinto service interactions."""


class MissingKintoDataFilesError(KintoError):
    """Error finding Kinto data files"""

    def __init__(self, kinto_data_dir: str):
        error_message: str = f"Cannot find Kinto data files in {kinto_data_dir}"
        super().__init__(error_message)
