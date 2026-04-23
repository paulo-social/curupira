import { Activity, Wifi, WifiOff } from 'lucide-react'

export function StatusCard({ online, lastSync }) {
  return (
    <section className="rounded-3xl border border-white/10 bg-white/5 p-6 shadow-glow backdrop-blur">
      <div className="mb-4 flex items-center justify-between">
        <div>
          <p className="text-sm uppercase tracking-[0.3em] text-forest-300/70">Status de Rede</p>
          <h2 className="mt-2 font-display text-2xl text-white">
            {online ? 'Backend acessível' : 'Sem resposta do backend'}
          </h2>
        </div>
        <div className={`rounded-2xl p-3 ${online ? 'bg-forest-500/20 text-forest-300' : 'bg-red-500/15 text-red-300'}`}>
          {online ? <Wifi size={28} /> : <WifiOff size={28} />}
        </div>
      </div>
      <div className="flex items-center gap-3 text-sm text-white/70">
        <Activity size={16} />
        <span>{lastSync ? `Última sincronização às ${lastSync}` : 'Aguardando primeira atualização'}</span>
      </div>
    </section>
  )
}

