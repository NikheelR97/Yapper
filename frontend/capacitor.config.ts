import type { CapacitorConfig } from '@capacitor/cli';

const config: CapacitorConfig = {
	appId: 'com.yapperhq.app',
	appName: 'Yapper',
	webDir: 'build',
	server: {
		// Allow CORS from Capacitor during dev
		androidScheme: 'https',
		cleartext: false,
	},
	plugins: {
		PushNotifications: {
			presentationOptions: ['badge', 'sound', 'alert'],
		},
		SplashScreen: {
			launchShowDuration: 0,
			backgroundColor: '#0F0F0F',
		},
	},
};

export default config;
