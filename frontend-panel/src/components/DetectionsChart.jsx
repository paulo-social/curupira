import { Bar, BarChart, CartesianGrid, ResponsiveContainer, Tooltip, XAxis, YAxis } from 'recharts'

export function DetectionsChart({ data }) {
  return (
    <section className="rounded-3xl border border-white/10 bg-white/5 p-6 shadow-glow backdrop-blur">
      <p className="text-sm uppercase tracking-[0.3em] text-forest-300/70">Volume de Detecções</p>
      <h2 className="mt-2 font-display text-2xl text-white">Ocorrências por hora</h2>

      <div className="mt-6 h-72">
        <ResponsiveContainer width="100%" height="100%">
          <BarChart data={data}>
            <CartesianGrid stroke="rgba(255,255,255,0.08)" vertical={false} />
            <XAxis dataKey="hour" stroke="#9fd4c0" />
            <YAxis stroke="#9fd4c0" allowDecimals={false} />
            <Tooltip
              contentStyle={{
                background: '#0b1d17',
                border: '1px solid rgba(159,212,192,0.15)',
                borderRadius: '16px'
              }}
            />
            <Bar dataKey="count" fill="#d9653b" radius={[8, 8, 0, 0]} />
          </BarChart>
        </ResponsiveContainer>
      </div>
    </section>
  )
}

