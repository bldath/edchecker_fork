example = '''
(<0>,1)                 None()                   SLP:{}
(<0>,2-3)               Load(Global(3)(8))       SLP:{}
(<0>,4)                 Spawn(2)                 SLP:{}
(<0>,5-6)               Load(Global(3)(8))       SLP:{}
(<0>,7)                 Post(4)                  SLP:{}
(<0>,8-9)               Load(Global(3)(8))       SLP:{}
(<0>,10)                Post(6)                  SLP:{}
(<0>,11)                Ret()                    SLP:{}
(<0.0>,1)           None()                   SLP:{}
(<0.0>,2-3)         Store(Stack(2,0)(8),0x0) SLP:{}
(<0.1>,1-3)     None()                   SLP:{} - (<0.2>,1-3)
(<0.1>,4)       Store(Stack(4,0)(8),0x0) SLP:{}   (<0.2>,4)
(<0.1>,5)       Store(Stack(4,1)(4),0x0) SLP:{}   (<0.2>,5)
(<0.1>,6)       Load(Stack(4,1)(4))      SLP:{}
(<0.1>,7)       Store(Global(1)(4),0x0)  SLP:{}
(<0.1>,8)       Store(Stack(4,2)(4),0x0) SLP:{}
(<0.1>,9)       Load(Stack(4,2)(4))      SLP:{}
(<0.1>,10)      Store(Global(2)(4),0x0)  SLP:{}
(<0.1>,11)      Ret()                    SLP:{}
(<0.2>,1-3) None()                   SLP:{}
(<0.2>,4)   Store(Stack(6,0)(8),0x0) SLP:{}
(<0.2>,5)   Load(Global(1)(4))       SLP:{}
(<0.2>,6)   Store(Stack(6,1)(4),0x0) SLP:{}
(<0.2>,7)   Load(Stack(6,1)(4))      SLP:{}
(<0.2>,8)   Load(Global(2)(4))       SLP:{}
(<0.2>,9)   Store(Stack(6,2)(4),0x0) SLP:{}
(<0.2>,10)  Load(Stack(6,2)(4))      SLP:{}
(<0.2>,11)  Ret()                    SLP:{}
'''
