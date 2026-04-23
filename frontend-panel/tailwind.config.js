export default {
  content: ['./index.html', './src/**/*.{js,jsx}'],
  theme: {
    extend: {
      colors: {
        forest: {
          950: '#07130f',
          900: '#0b1d17',
          800: '#123126',
          700: '#1d4d3a',
          500: '#4f9c7f',
          300: '#9fd4c0'
        },
        ember: {
          500: '#d9653b',
          600: '#bb4721'
        }
      },
      boxShadow: {
        glow: '0 0 0 1px rgba(159, 212, 192, 0.18), 0 16px 40px rgba(0, 0, 0, 0.35)'
      },
      keyframes: {
        pulseAlert: {
          '0%, 100%': { opacity: '0.82', transform: 'scale(1)' },
          '50%': { opacity: '1', transform: 'scale(1.01)' }
        },
        slideIn: {
          from: { opacity: '0', transform: 'translateY(10px)' },
          to: { opacity: '1', transform: 'translateY(0)' }
        }
      },
      animation: {
        alert: 'pulseAlert 1.2s ease-in-out infinite',
        rise: 'slideIn 500ms ease-out both'
      },
      fontFamily: {
        display: ['"Space Grotesk"', 'sans-serif'],
        body: ['"Manrope"', 'sans-serif']
      }
    }
  },
  plugins: []
}

