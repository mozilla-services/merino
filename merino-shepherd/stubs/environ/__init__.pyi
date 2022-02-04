from pathlib import Path
from typing import TypeVar, Type

T = TypeVar('T')


class NoValue:
    ...


class Env:
    @classmethod
    def read_env(cls, path: Path): ...

    @classmethod
    def db(cls, default: str | Type[NoValue] = NoValue): ...

    def __call__(self, name: str, default: T | Type[NoValue] = NoValue) -> T:
        ...
