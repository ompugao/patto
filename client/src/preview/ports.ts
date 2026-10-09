import * as net from 'net';

function isPortAvailable(port: number): Promise<boolean> {
	return new Promise((resolve) => {
		const server = net.createServer();
		server.once('error', () => resolve(false));
		server.once('listening', () => {
			server.close();
			resolve(true);
		});
		server.listen(port);
	});
}

export async function findAvailablePort(startPort: number = 3000, maxAttempts: number = 100): Promise<number | null> {
	for (let i = 0; i < maxAttempts; i++) {
		const port = startPort + i;
		if (await isPortAvailable(port)) {
			return port;
		}
	}
	return null;
}
