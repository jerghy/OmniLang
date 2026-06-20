# Generated from input.omni

from dataclasses import dataclass

@dataclass
class Post:
    title: str
    body: str
    author: str

def format_date(timestamp: int) -> str:
    return '2026-06-17'

class PostCard:
    def __init__(self):
        self.title = ''
        self.body = ''
        self.author = ''


class App:
    def __init__(self):
        self.posts = ['First Post', 'Second Post']


