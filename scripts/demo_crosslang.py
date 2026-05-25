from pymergetic.cruspy.models.hello import Hello

h = Hello(message="cruspy")
print(h.hello_cpp().decode())
print(h.hello_rust().decode())
print(h.hello_python().decode())
