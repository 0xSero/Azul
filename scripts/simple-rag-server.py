#!/usr/bin/env python3
"""
Simple RAG endpoint for azul-browse
Provides a minimal RAG API without heavy dependencies
"""

from http.server import HTTPServer, BaseHTTPRequestHandler
import json
import os
from pathlib import Path

class SimpleRAGHandler(BaseHTTPRequestHandler):
    # Simple in-memory document store
    documents = []

    def do_GET(self):
        if self.path == '/health':
            self.send_response(200)
            self.send_header('Content-type', 'application/json')
            self.end_headers()
            self.wfile.write(json.dumps({"status": "ok"}).encode())
        else:
            self.send_response(404)
            self.end_headers()

    def do_POST(self):
        if self.path == '/query':
            content_length = int(self.headers['Content-Length'])
            post_data = self.rfile.read(content_length)

            try:
                request = json.loads(post_data.decode('utf-8'))
                query = request.get('query', '')
                top_k = request.get('top_k', 5)

                # Simple keyword matching (no embeddings needed)
                results = self.search_documents(query, top_k)

                response = {
                    'answer': None,
                    'sources': results,
                    'context': [r['content'] for r in results]
                }

                self.send_response(200)
                self.send_header('Content-type', 'application/json')
                self.end_headers()
                self.wfile.write(json.dumps(response).encode())
            except Exception as e:
                self.send_error(500, str(e))
        elif self.path == '/ingest':
            content_length = int(self.headers['Content-Length'])
            post_data = self.rfile.read(content_length)

            try:
                request = json.loads(post_data.decode('utf-8'))
                docs = request.get('documents', [])

                for doc in docs:
                    self.documents.append({
                        'content': doc.get('content', ''),
                        'metadata': doc.get('metadata', {}),
                        'score': 1.0
                    })

                self.send_response(200)
                self.send_header('Content-type', 'application/json')
                self.end_headers()
                self.wfile.write(json.dumps({
                    'status': 'ok',
                    'count': len(docs)
                }).encode())
            except Exception as e:
                self.send_error(500, str(e))
        else:
            self.send_response(404)
            self.end_headers()

    @classmethod
    def search_documents(cls, query, top_k):
        """Simple keyword-based search"""
        query_lower = query.lower()
        query_words = set(query_lower.split())

        # Score documents by keyword overlap
        scored_docs = []
        for doc in cls.documents:
            content_lower = doc['content'].lower()
            content_words = set(content_lower.split())

            # Simple relevance: count matching words
            matches = len(query_words & content_words)
            if matches > 0:
                score = matches / len(query_words)
                scored_docs.append({
                    'content': doc['content'][:200],  # Truncate for display
                    'metadata': doc['metadata'],
                    'score': score
                })

        # Sort by score and return top_k
        scored_docs.sort(key=lambda x: x['score'], reverse=True)
        return scored_docs[:top_k]

    def log_message(self, format, *args):
        # Suppress default logging
        pass

def load_sample_documents():
    """Load some sample documents for testing"""
    SimpleRAGHandler.documents = [
        {
            'content': 'Azul is a terminal web browser written in Rust with AI-powered features including chat, RAG, and memory integration.',
            'metadata': {'source': 'README', 'type': 'project'},
            'score': 1.0
        },
        {
            'content': 'mem-layer is a graph-based memory management system for AI models. It stores nodes and relationships in a graph database.',
            'metadata': {'source': 'docs', 'type': 'integration'},
            'score': 1.0
        },
        {
            'content': 'The browser supports multiple search engines including DuckDuckGo, Wikipedia, arXiv, and Google Scholar.',
            'metadata': {'source': 'features', 'type': 'documentation'},
            'score': 1.0
        },
        {
            'content': 'Press r to open RAG panel, m for memory panel, c for chat, and ? for help.',
            'metadata': {'source': 'keybindings', 'type': 'documentation'},
            'score': 1.0
        },
    ]

def run_server(port=8765):
    """Run the simple RAG server"""
    load_sample_documents()

    server_address = ('127.0.0.1', port)
    httpd = HTTPServer(server_address, SimpleRAGHandler)

    print(f'Simple RAG server running on http://127.0.0.1:{port}')
    print('Endpoints:')
    print('  GET  /health - Health check')
    print('  POST /query  - Search documents')
    print('  POST /ingest - Add documents')
    print('\nPress Ctrl+C to stop')

    try:
        httpd.serve_forever()
    except KeyboardInterrupt:
        print('\nShutting down...')
        httpd.shutdown()

if __name__ == '__main__':
    import sys
    port = int(sys.argv[1]) if len(sys.argv) > 1 else 8766
    run_server(port)
