import { useEffect, useState } from 'react'
import { AlertOctagon, LayoutDashboard, ShieldAlert } from 'lucide-react'
import { AlertsList } from './components/AlertsList'
import { DetectionsChart } from './components/DetectionsChart'
import { StatusCard } from './components/StatusCard'

const API_BASE = import.meta.env.VITE_API_URL || 'http://localhost:8080'

export function isCriticalAlert(alert) {
  return Boolean(alert && alert.confianca > 80)
}

export function buildChartData(alerts) {
  const grouped = alerts.reduce((acc, alert) => {
    const hour = new Date(alert.timestamp).toLocaleTimeString('pt-BR', {
      hour: '2-digit'
    })
    acc[hour] = (acc[hour] || 0) + 1
    return acc
  }, {})

  return Object.entries(grouped)
    .map(([hour, count]) => ({ hour: `${hour}h`, count }))
    .sort((a, b) => a.hour.localeCompare(b.hour))
}

export default function App() {
  const [alerts, setAlerts] = useState([])
  const [online, setOnline] = useState(false)
  const [lastSync, setLastSync] = useState('')

  useEffect(() => {
    let active = true

    const loadAlerts = async () => {
      try {
        const response = await fetch(`${API_BASE}/alerts`)
        if (!response.ok) {
          throw new Error('backend indisponível')
        }

        const data = await response.json()
        if (!active) {
          return
        }

        setAlerts(data)
        setOnline(true)
        setLastSync(
          new Date().toLocaleTimeString('pt-BR', {
            hour: '2-digit',
            minute: '2-digit',
            second: '2-digit'
          })
        )
      } catch (error) {
        if (!active) {
          return
        }
        setOnline(false)
      }
    }

    loadAlerts()
    const interval = window.setInterval(loadAlerts, 5000)

    return () => {
      active = false
      window.clearInterval(interval)
    }
  }, [])

  const latestAlert = alerts[0]
  const isCritical = isCriticalAlert(latestAlert)
  const chartData = buildChartData(alerts)

  return (
    <div className="min-h-screen bg-transparent text-white">
      <div className="mx-auto grid min-h-screen max-w-7xl gap-6 px-4 py-6 lg:grid-cols-[280px_minmax(0,1fr)]">
        <aside className="animate-rise rounded-[2rem] border border-white/10 bg-black/25 p-6 shadow-glow backdrop-blur">
          <div className="flex items-center gap-3">
            <div className="rounded-2xl bg-forest-500/20 p-3 text-forest-300">
              <ShieldAlert />
            </div>
            <div>
              <p className="font-display text-2xl">Curupira</p>
              <p className="text-sm text-white/60">Monitoramento de impacto ambiental</p>
            </div>
          </div>

          <nav className="mt-10 space-y-3 text-sm text-white/70">
            <div className="flex items-center gap-3 rounded-2xl bg-white/10 px-4 py-3 text-white">
              <LayoutDashboard size={18} />
              Painel principal
            </div>
            <div className="flex items-center gap-3 rounded-2xl px-4 py-3">
              <AlertOctagon size={18} />
              Zona crítica
            </div>
          </nav>
        </aside>

        <main className="space-y-6">
          {isCritical ? (
            <section className="animate-alert rounded-3xl border border-red-300/30 bg-red-500/85 px-6 py-5 text-white shadow-2xl">
              <p className="text-sm uppercase tracking-[0.35em] text-red-100">Alerta Crítico</p>
              <h1 className="mt-2 font-display text-3xl">ATIVIDADE SUSPEITA DETECTADA</h1>
              <p className="mt-2 text-red-50/90">
                Último evento: <span className="font-bold capitalize">{latestAlert.tipo_evento}</span> com{' '}
                {Math.round(latestAlert.confianca)}% de confiança.
              </p>
            </section>
          ) : null}

          <section className="grid gap-6 xl:grid-cols-[minmax(0,1fr)_360px]">
            <DetectionsChart data={chartData} />
            <StatusCard online={online} lastSync={lastSync} />
          </section>

          <AlertsList alerts={alerts} />
        </main>
      </div>
    </div>
  )
}
